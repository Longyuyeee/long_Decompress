use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use zeroize::{Zeroize, Zeroizing};

pub const AES_MAGIC_V2: &[u8; 8] = b"AESENC02";
pub const TAR_AES_MAGIC_V2: &[u8; 8] = b"TARAES02";

const HEADER_SIZE: usize = 76;
const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;
const MIN_CHUNK_SIZE: usize = 64 * 1024;
const MAX_CHUNK_SIZE: usize = 16 * 1024 * 1024;
const TAG_SIZE: u64 = 16;
const SALT_SIZE: usize = 32;
const NONCE_PREFIX_SIZE: usize = 4;
const ARGON_MEMORY_KIB: u32 = 65_536;
const ARGON_ITERATIONS: u32 = 3;
const ARGON_PARALLELISM: u32 = 1;
const KDF_ARGON2ID: u8 = 1;
const CIPHER_AES_256_GCM: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AesStreamKind {
    Generic,
    Tar,
}

impl AesStreamKind {
    fn magic(self) -> &'static [u8; 8] {
        match self {
            Self::Generic => AES_MAGIC_V2,
            Self::Tar => TAR_AES_MAGIC_V2,
        }
    }
}

struct Header {
    encoded: [u8; HEADER_SIZE],
    plaintext_len: u64,
    chunk_size: usize,
    salt: [u8; SALT_SIZE],
    nonce_prefix: [u8; NONCE_PREFIX_SIZE],
}

impl Header {
    fn new(kind: AesStreamKind, plaintext_len: u64, chunk_size: usize) -> Result<Self> {
        validate_chunk_size(chunk_size)?;

        let mut salt = [0u8; SALT_SIZE];
        let mut nonce_prefix = [0u8; NONCE_PREFIX_SIZE];
        rand::thread_rng().fill_bytes(&mut salt);
        rand::thread_rng().fill_bytes(&mut nonce_prefix);

        let mut encoded = [0u8; HEADER_SIZE];
        encoded[0..8].copy_from_slice(kind.magic());
        encoded[8..10].copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
        encoded[10..12].copy_from_slice(&0u16.to_le_bytes());
        encoded[12..16].copy_from_slice(&(chunk_size as u32).to_le_bytes());
        encoded[16..24].copy_from_slice(&plaintext_len.to_le_bytes());
        encoded[24] = KDF_ARGON2ID;
        encoded[25] = CIPHER_AES_256_GCM;
        encoded[26..28].copy_from_slice(&0u16.to_le_bytes());
        encoded[28..32].copy_from_slice(&ARGON_MEMORY_KIB.to_le_bytes());
        encoded[32..36].copy_from_slice(&ARGON_ITERATIONS.to_le_bytes());
        encoded[36..40].copy_from_slice(&ARGON_PARALLELISM.to_le_bytes());
        encoded[40..72].copy_from_slice(&salt);
        encoded[72..76].copy_from_slice(&nonce_prefix);

        Ok(Self {
            encoded,
            plaintext_len,
            chunk_size,
            salt,
            nonce_prefix,
        })
    }

    fn read(reader: &mut impl Read, kind: AesStreamKind) -> Result<Self> {
        let mut encoded = [0u8; HEADER_SIZE];
        reader
            .read_exact(&mut encoded)
            .context("AES v2 文件头被截断")?;

        if &encoded[0..8] != kind.magic() {
            return Err(anyhow!("无效的 AES v2 文件魔数"));
        }
        if u16::from_le_bytes(encoded[8..10].try_into()?) as usize != HEADER_SIZE {
            return Err(anyhow!("不支持的 AES v2 文件头长度"));
        }
        if u16::from_le_bytes(encoded[10..12].try_into()?) != 0
            || u16::from_le_bytes(encoded[26..28].try_into()?) != 0
        {
            return Err(anyhow!("AES v2 文件包含不支持的标志或保留字段"));
        }
        if encoded[24] != KDF_ARGON2ID || encoded[25] != CIPHER_AES_256_GCM {
            return Err(anyhow!("AES v2 文件使用了不支持的 KDF 或加密算法"));
        }

        let chunk_size = u32::from_le_bytes(encoded[12..16].try_into()?) as usize;
        validate_chunk_size(chunk_size)?;
        let plaintext_len = u64::from_le_bytes(encoded[16..24].try_into()?);
        let memory = u32::from_le_bytes(encoded[28..32].try_into()?);
        let iterations = u32::from_le_bytes(encoded[32..36].try_into()?);
        let parallelism = u32::from_le_bytes(encoded[36..40].try_into()?);
        if (memory, iterations, parallelism)
            != (ARGON_MEMORY_KIB, ARGON_ITERATIONS, ARGON_PARALLELISM)
        {
            return Err(anyhow!("AES v2 文件使用了不支持的 Argon2 参数"));
        }

        let mut salt = [0u8; SALT_SIZE];
        salt.copy_from_slice(&encoded[40..72]);
        let mut nonce_prefix = [0u8; NONCE_PREFIX_SIZE];
        nonce_prefix.copy_from_slice(&encoded[72..76]);

        Ok(Self {
            encoded,
            plaintext_len,
            chunk_size,
            salt,
            nonce_prefix,
        })
    }

    fn chunk_count(&self) -> u64 {
        if self.plaintext_len == 0 {
            1
        } else {
            ((self.plaintext_len - 1) / self.chunk_size as u64) + 1
        }
    }

    fn expected_container_len(&self) -> Result<u64> {
        self.plaintext_len
            .checked_add(
                self.chunk_count()
                    .checked_mul(TAG_SIZE)
                    .ok_or_else(|| anyhow!("AES v2 分块数量溢出"))?,
            )
            .and_then(|size| size.checked_add(HEADER_SIZE as u64))
            .ok_or_else(|| anyhow!("AES v2 容器大小溢出"))
    }

    fn plaintext_chunk_len(&self, index: u64) -> Result<usize> {
        if index >= self.chunk_count() {
            return Err(anyhow!("AES v2 分块索引越界"));
        }
        if self.plaintext_len == 0 {
            return Ok(0);
        }
        let offset = index
            .checked_mul(self.chunk_size as u64)
            .ok_or_else(|| anyhow!("AES v2 分块偏移溢出"))?;
        usize::try_from((self.plaintext_len - offset).min(self.chunk_size as u64))
            .context("AES v2 分块长度无法表示")
    }

    fn nonce(&self, index: u64) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[..NONCE_PREFIX_SIZE].copy_from_slice(&self.nonce_prefix);
        nonce[NONCE_PREFIX_SIZE..].copy_from_slice(&index.to_be_bytes());
        nonce
    }

    fn aad(&self, index: u64) -> [u8; HEADER_SIZE + 8] {
        let mut aad = [0u8; HEADER_SIZE + 8];
        aad[..HEADER_SIZE].copy_from_slice(&self.encoded);
        aad[HEADER_SIZE..].copy_from_slice(&index.to_be_bytes());
        aad
    }
}

struct IncompleteOutput {
    path: PathBuf,
    committed: bool,
}

impl IncompleteOutput {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            committed: false,
        }
    }
}

impl Drop for IncompleteOutput {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn write_new_output<W: Write>(
    output: &Path,
    mut writer: W,
    write_contents: impl FnOnce(&mut W) -> Result<()>,
    commit: impl FnOnce(&mut W) -> Result<()>,
) -> Result<()> {
    let mut cleanup = IncompleteOutput::new(output);
    let result = (|| {
        write_contents(&mut writer)?;
        commit(&mut writer)
    })();
    if result.is_ok() {
        cleanup.committed = true;
    }
    drop(writer);
    result
}

pub struct AesStreamV2;

impl AesStreamV2 {
    pub fn encrypt_file(
        input: &Path,
        output: &Path,
        password: &str,
        kind: AesStreamKind,
    ) -> Result<()> {
        Self::encrypt_file_cancellable(input, output, password, kind, || Ok(()))
    }

    pub fn encrypt_file_cancellable(
        input: &Path,
        output: &Path,
        password: &str,
        kind: AesStreamKind,
        mut check_cancellation: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        Self::encrypt_file_with_chunk_size_cancellable(
            input,
            output,
            password,
            kind,
            DEFAULT_CHUNK_SIZE,
            &mut check_cancellation,
        )
    }

    #[cfg(test)]
    fn encrypt_file_with_chunk_size(
        input: &Path,
        output: &Path,
        password: &str,
        kind: AesStreamKind,
        chunk_size: usize,
    ) -> Result<()> {
        Self::encrypt_file_with_chunk_size_cancellable(
            input,
            output,
            password,
            kind,
            chunk_size,
            &mut || Ok(()),
        )
    }

    fn encrypt_file_with_chunk_size_cancellable(
        input: &Path,
        output: &Path,
        password: &str,
        kind: AesStreamKind,
        chunk_size: usize,
        check_cancellation: &mut impl FnMut() -> Result<()>,
    ) -> Result<()> {
        check_cancellation()?;
        let input_file =
            File::open(input).with_context(|| format!("打开输入文件失败: {}", input.display()))?;
        let plaintext_len = input_file.metadata()?.len();
        let header = Header::new(kind, plaintext_len, chunk_size)?;
        let cipher = derive_cipher(password, &header.salt)?;
        check_cancellation()?;

        let output_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .with_context(|| format!("创建 AES v2 输出失败: {}", output.display()))?;
        let mut reader = BufReader::new(input_file);
        write_new_output(
            output,
            BufWriter::new(output_file),
            |writer| {
                Self::encrypt_to_writer(&mut reader, &header, &cipher, writer, check_cancellation)
            },
            |writer| {
                writer.flush()?;
                writer.get_ref().sync_all()?;
                Ok(())
            },
        )
    }

    pub fn decrypt_file(
        input: &Path,
        output: &Path,
        password: &str,
        kind: AesStreamKind,
    ) -> Result<()> {
        Self::decrypt_file_cancellable(input, output, password, kind, || Ok(()))
    }

    pub fn decrypt_file_cancellable(
        input: &Path,
        output: &Path,
        password: &str,
        kind: AesStreamKind,
        mut check_cancellation: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        let output_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .with_context(|| format!("创建 AES v2 解密输出失败: {}", output.display()))?;
        write_new_output(
            output,
            BufWriter::new(output_file),
            |writer| {
                Self::decrypt_to_writer(input, password, kind, writer, &mut check_cancellation)?;
                check_cancellation()
            },
            |writer| {
                writer.flush()?;
                writer.get_ref().sync_all()?;
                Ok(())
            },
        )
    }

    pub fn verify_password(input: &Path, password: &str, kind: AesStreamKind) -> Result<bool> {
        Self::verify_password_cancellable(input, password, kind, || Ok(()))
    }

    pub fn verify_password_cancellable(
        input: &Path,
        password: &str,
        kind: AesStreamKind,
        mut check_cancellation: impl FnMut() -> Result<()>,
    ) -> Result<bool> {
        match Self::decrypt_to_writer(
            input,
            password,
            kind,
            &mut std::io::sink(),
            &mut check_cancellation,
        ) {
            Ok(()) => Ok(true),
            Err(error) if error.to_string().contains("密码错误或文件已损坏") => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn is_kind(path: &Path, kind: AesStreamKind) -> Result<bool> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 8];
        if file.read_exact(&mut magic).is_err() {
            return Ok(false);
        }
        Ok(&magic == kind.magic())
    }

    fn encrypt_to_writer(
        reader: &mut impl Read,
        header: &Header,
        cipher: &Aes256Gcm,
        writer: &mut impl Write,
        check_cancellation: &mut impl FnMut() -> Result<()>,
    ) -> Result<()> {
        writer.write_all(&header.encoded)?;

        let mut plaintext = Zeroizing::new(vec![0u8; header.chunk_size]);
        for index in 0..header.chunk_count() {
            check_cancellation()?;
            let length = header.plaintext_chunk_len(index)?;
            if length > 0 {
                reader
                    .read_exact(&mut plaintext[..length])
                    .context("读取 AES v2 输入分块失败")?;
            }
            let nonce_bytes = header.nonce(index);
            let aad = header.aad(index);
            let ciphertext = cipher
                .encrypt(
                    Nonce::from_slice(&nonce_bytes),
                    Payload {
                        msg: &plaintext[..length],
                        aad: &aad,
                    },
                )
                .map_err(|_| anyhow!("AES v2 分块加密失败"))?;
            writer.write_all(&ciphertext)?;
        }
        check_cancellation()
    }

    fn decrypt_to_writer(
        input: &Path,
        password: &str,
        kind: AesStreamKind,
        writer: &mut impl Write,
        check_cancellation: &mut impl FnMut() -> Result<()>,
    ) -> Result<()> {
        check_cancellation()?;
        let input_file = File::open(input)
            .with_context(|| format!("打开 AES v2 文件失败: {}", input.display()))?;
        let actual_len = input_file.metadata()?.len();
        let mut reader = BufReader::new(input_file);
        let header = Header::read(&mut reader, kind)?;
        if actual_len != header.expected_container_len()? {
            return Err(anyhow!("AES v2 文件长度不匹配，文件可能被截断或附加了数据"));
        }
        let cipher = derive_cipher(password, &header.salt)?;
        check_cancellation()?;

        let mut ciphertext = vec![0u8; header.chunk_size + TAG_SIZE as usize];
        for index in 0..header.chunk_count() {
            check_cancellation()?;
            let plaintext_len = header.plaintext_chunk_len(index)?;
            let ciphertext_len = plaintext_len + TAG_SIZE as usize;
            reader
                .read_exact(&mut ciphertext[..ciphertext_len])
                .context("AES v2 密文分块被截断")?;
            let nonce_bytes = header.nonce(index);
            let aad = header.aad(index);
            let plaintext = Zeroizing::new(
                cipher
                    .decrypt(
                        Nonce::from_slice(&nonce_bytes),
                        Payload {
                            msg: &ciphertext[..ciphertext_len],
                            aad: &aad,
                        },
                    )
                    .map_err(|_| anyhow!("AES v2 解密失败: 密码错误或文件已损坏"))?,
            );
            writer.write_all(&plaintext)?;
        }
        check_cancellation()?;
        ciphertext.zeroize();
        Ok(())
    }
}

fn validate_chunk_size(chunk_size: usize) -> Result<()> {
    if !(MIN_CHUNK_SIZE..=MAX_CHUNK_SIZE).contains(&chunk_size) || !chunk_size.is_power_of_two() {
        return Err(anyhow!("AES v2 分块大小无效"));
    }
    Ok(())
}

fn derive_cipher(password: &str, salt: &[u8; SALT_SIZE]) -> Result<Aes256Gcm> {
    let params = Params::new(
        ARGON_MEMORY_KIB,
        ARGON_ITERATIONS,
        ARGON_PARALLELISM,
        Some(32),
    )
    .map_err(|error| anyhow!("创建 Argon2 参数失败: {error:?}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|error| anyhow!("密钥派生失败: {error:?}"))?;
    let cipher = Aes256Gcm::new_from_slice(&key).context("创建 AES v2 加密器失败")?;
    key.zeroize();
    Ok(cipher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Error, ErrorKind};
    use tempfile::TempDir;

    struct StorageFullWriter<W> {
        inner: W,
        remaining: usize,
    }

    impl<W> StorageFullWriter<W> {
        fn new(inner: W, remaining: usize) -> Self {
            Self { inner, remaining }
        }
    }

    impl<W: Write> Write for StorageFullWriter<W> {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            if self.remaining == 0 {
                return Err(Error::new(ErrorKind::StorageFull, "simulated storage full"));
            }
            let allowed = buffer.len().min(self.remaining);
            let written = self.inner.write(&buffer[..allowed])?;
            self.remaining -= written;
            Ok(written)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }

    fn assert_storage_full(error: &anyhow::Error) {
        assert_eq!(
            error
                .downcast_ref::<std::io::Error>()
                .expect("storage-full error must remain an I/O error")
                .kind(),
            ErrorKind::StorageFull
        );
    }

    #[test]
    fn multi_chunk_roundtrip_is_bounded_and_exact() -> Result<()> {
        let temp = TempDir::new()?;
        let input = temp.path().join("input.bin");
        let encrypted = temp.path().join("encrypted.aes");
        let output = temp.path().join("output.bin");
        let data: Vec<u8> = (0..(MIN_CHUNK_SIZE * 2 + 137))
            .map(|index| (index % 251) as u8)
            .collect();
        fs::write(&input, &data)?;

        AesStreamV2::encrypt_file_with_chunk_size(
            &input,
            &encrypted,
            "correct horse battery staple",
            AesStreamKind::Generic,
            MIN_CHUNK_SIZE,
        )?;
        AesStreamV2::decrypt_file(
            &encrypted,
            &output,
            "correct horse battery staple",
            AesStreamKind::Generic,
        )?;

        assert_eq!(fs::read(output)?, data);
        Ok(())
    }

    #[test]
    fn empty_file_roundtrip_has_an_authenticated_final_chunk() -> Result<()> {
        let temp = TempDir::new()?;
        let input = temp.path().join("empty.bin");
        let encrypted = temp.path().join("empty.aes");
        let output = temp.path().join("output.bin");
        fs::write(&input, b"")?;

        AesStreamV2::encrypt_file(&input, &encrypted, "password", AesStreamKind::Generic)?;
        AesStreamV2::decrypt_file(&encrypted, &output, "password", AesStreamKind::Generic)?;

        assert_eq!(fs::metadata(output)?.len(), 0);
        assert_eq!(
            fs::metadata(encrypted)?.len(),
            HEADER_SIZE as u64 + TAG_SIZE
        );
        Ok(())
    }

    #[test]
    fn wrong_password_removes_partial_output() -> Result<()> {
        let temp = TempDir::new()?;
        let input = temp.path().join("input.bin");
        let encrypted = temp.path().join("encrypted.aes");
        let output = temp.path().join("output.bin");
        fs::write(&input, b"secret")?;
        AesStreamV2::encrypt_file(&input, &encrypted, "right", AesStreamKind::Generic)?;

        let error = AesStreamV2::decrypt_file(&encrypted, &output, "wrong", AesStreamKind::Generic)
            .unwrap_err();
        assert!(error.to_string().contains("密码错误"));
        assert!(!output.exists());
        Ok(())
    }

    #[test]
    fn tampering_and_truncation_are_rejected() -> Result<()> {
        let temp = TempDir::new()?;
        let input = temp.path().join("input.bin");
        let encrypted = temp.path().join("encrypted.aes");
        fs::write(&input, vec![7u8; MIN_CHUNK_SIZE + 1])?;
        AesStreamV2::encrypt_file_with_chunk_size(
            &input,
            &encrypted,
            "password",
            AesStreamKind::Generic,
            MIN_CHUNK_SIZE,
        )?;

        let mut tampered = fs::read(&encrypted)?;
        tampered[HEADER_SIZE + 3] ^= 0x80;
        let tampered_path = temp.path().join("tampered.aes");
        fs::write(&tampered_path, tampered)?;
        assert!(!AesStreamV2::verify_password(
            &tampered_path,
            "password",
            AesStreamKind::Generic,
        )?);

        let mut truncated = fs::read(&encrypted)?;
        truncated.pop();
        let truncated_path = temp.path().join("truncated.aes");
        fs::write(&truncated_path, truncated)?;
        let error =
            AesStreamV2::verify_password(&truncated_path, "password", AesStreamKind::Generic)
                .unwrap_err();
        assert!(error.to_string().contains("长度不匹配"));
        Ok(())
    }

    #[test]
    fn cancellation_removes_partial_encrypted_and_decrypted_outputs() -> Result<()> {
        let temp = TempDir::new()?;
        let input = temp.path().join("input.bin");
        let encrypted = temp.path().join("encrypted.aes");
        let cancelled_encrypted = temp.path().join("cancelled.aes");
        let cancelled_decrypted = temp.path().join("cancelled.bin");
        fs::write(&input, vec![9u8; MIN_CHUNK_SIZE * 3])?;

        let mut encrypt_checks = 0usize;
        let encrypt_error = AesStreamV2::encrypt_file_with_chunk_size_cancellable(
            &input,
            &cancelled_encrypted,
            "password",
            AesStreamKind::Generic,
            MIN_CHUNK_SIZE,
            &mut || {
                encrypt_checks += 1;
                if encrypt_checks == 4 {
                    Err(anyhow!("test cancellation"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert!(encrypt_error.to_string().contains("cancellation"));
        assert!(!cancelled_encrypted.exists());

        AesStreamV2::encrypt_file_with_chunk_size(
            &input,
            &encrypted,
            "password",
            AesStreamKind::Generic,
            MIN_CHUNK_SIZE,
        )?;
        let mut decrypt_checks = 0usize;
        let decrypt_error = AesStreamV2::decrypt_file_cancellable(
            &encrypted,
            &cancelled_decrypted,
            "password",
            AesStreamKind::Generic,
            || {
                decrypt_checks += 1;
                if decrypt_checks == 4 {
                    Err(anyhow!("test cancellation"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert!(decrypt_error.to_string().contains("cancellation"));
        assert!(!cancelled_decrypted.exists());
        Ok(())
    }

    #[test]
    fn storage_full_errors_remove_partial_and_uncommitted_outputs() -> Result<()> {
        let temp = TempDir::new()?;
        let input = temp.path().join("input.bin");
        let encrypted = temp.path().join("encrypted.aes");
        let partial_encrypted = temp.path().join("partial.aes");
        let partial_decrypted = temp.path().join("partial.bin");
        let uncommitted_decrypted = temp.path().join("uncommitted.bin");
        fs::write(&input, vec![11u8; MIN_CHUNK_SIZE * 2])?;

        let input_file = File::open(&input)?;
        let header = Header::new(
            AesStreamKind::Generic,
            input_file.metadata()?.len(),
            MIN_CHUNK_SIZE,
        )?;
        let cipher = derive_cipher("password", &header.salt)?;
        let mut reader = BufReader::new(input_file);
        let partial_output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial_encrypted)?;
        let encryption_error = write_new_output(
            &partial_encrypted,
            StorageFullWriter::new(
                BufWriter::new(partial_output),
                HEADER_SIZE + MIN_CHUNK_SIZE / 2,
            ),
            |writer| {
                AesStreamV2::encrypt_to_writer(
                    &mut reader,
                    &header,
                    &cipher,
                    writer,
                    &mut || Ok(()),
                )
            },
            |writer| {
                writer.flush()?;
                Ok(())
            },
        )
        .unwrap_err();
        assert_storage_full(&encryption_error);
        assert!(!partial_encrypted.exists());

        AesStreamV2::encrypt_file_with_chunk_size(
            &input,
            &encrypted,
            "password",
            AesStreamKind::Generic,
            MIN_CHUNK_SIZE,
        )?;

        let partial_output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial_decrypted)?;
        let decryption_error = write_new_output(
            &partial_decrypted,
            StorageFullWriter::new(BufWriter::new(partial_output), MIN_CHUNK_SIZE / 2),
            |writer| {
                AesStreamV2::decrypt_to_writer(
                    &encrypted,
                    "password",
                    AesStreamKind::Generic,
                    writer,
                    &mut || Ok(()),
                )
            },
            |writer| {
                writer.flush()?;
                Ok(())
            },
        )
        .unwrap_err();
        assert_storage_full(&decryption_error);
        assert!(!partial_decrypted.exists());

        let uncommitted_output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&uncommitted_decrypted)?;
        let commit_error = write_new_output(
            &uncommitted_decrypted,
            BufWriter::new(uncommitted_output),
            |writer| {
                AesStreamV2::decrypt_to_writer(
                    &encrypted,
                    "password",
                    AesStreamKind::Generic,
                    writer,
                    &mut || Ok(()),
                )
            },
            |writer| {
                writer.flush()?;
                Err(Error::new(ErrorKind::StorageFull, "simulated fsync storage full").into())
            },
        )
        .unwrap_err();
        assert_storage_full(&commit_error);
        assert!(!uncommitted_decrypted.exists());
        Ok(())
    }
}
