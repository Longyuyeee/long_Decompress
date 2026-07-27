use crate::services::aes_stream_v2::{AesStreamKind, AesStreamV2, TAR_AES_MAGIC_V2};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Builder;
use zeroize::{Zeroize, Zeroizing};

/// TAR + AES-256-GCM 引擎。
///
/// 新文件使用分块的 TARAES02；TARAES01 仅保留受限的只读兼容。
pub struct TarAesEngine;

const LEGACY_MAGIC: &[u8; 8] = b"TARAES01";
const SALT_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;
const MAX_LEGACY_TAR_AES_BYTES: u64 = 512 * 1024 * 1024;

struct TemporaryPath(PathBuf);

impl TemporaryPath {
    fn near(destination: &Path, extension: &str) -> Self {
        let parent = destination.parent().unwrap_or_else(|| Path::new("."));
        Self(parent.join(format!(
            "long-compress-{}.{}",
            uuid::Uuid::new_v4(),
            extension
        )))
    }

    fn as_path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

impl TarAesEngine {
    pub fn compress_tar_aes(
        files: &[PathBuf],
        output: &Path,
        password: &str,
        base_dir: Option<&Path>,
    ) -> Result<()> {
        Self::compress_tar_aes_cancellable(files, output, password, base_dir, || Ok(()))
    }

    pub fn compress_tar_aes_cancellable(
        files: &[PathBuf],
        output: &Path,
        password: &str,
        base_dir: Option<&Path>,
        mut check_cancellation: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        Self::validate_sources(files, &mut check_cancellation)?;
        let temporary_tar = TemporaryPath::near(output, "tar");
        Self::create_tar_archive(
            files,
            base_dir,
            temporary_tar.as_path(),
            &mut check_cancellation,
        )
        .context("创建 TAR 归档失败")?;
        AesStreamV2::encrypt_file_cancellable(
            temporary_tar.as_path(),
            output,
            password,
            AesStreamKind::Tar,
            check_cancellation,
        )
        .context("AES v2 加密失败")
    }

    pub fn decompress_tar_aes(
        archive_path: &Path,
        output_dir: &Path,
        password: &str,
    ) -> Result<()> {
        Self::decompress_tar_aes_cancellable(archive_path, output_dir, password, || Ok(()))
    }

    pub fn decompress_tar_aes_cancellable(
        archive_path: &Path,
        output_dir: &Path,
        password: &str,
        mut check_cancellation: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        if AesStreamV2::is_kind(archive_path, AesStreamKind::Tar)? {
            let temporary_tar = TemporaryPath::near(output_dir, "decrypted.tar");
            AesStreamV2::decrypt_file_cancellable(
                archive_path,
                temporary_tar.as_path(),
                password,
                AesStreamKind::Tar,
                &mut check_cancellation,
            )
            .context("AES v2 解密失败")?;
            return Self::extract_tar_file(
                temporary_tar.as_path(),
                output_dir,
                &mut check_cancellation,
            )
            .context("解压 TAR 归档失败");
        }

        let tar_data = Self::decrypt_legacy_data(archive_path, password, &mut check_cancellation)
            .context("AES 解密失败")?;
        Self::extract_tar_reader(tar_data.as_slice(), output_dir, &mut check_cancellation)
            .context("解压 TAR 归档失败")
    }

    fn validate_sources(
        files: &[PathBuf],
        check_cancellation: &mut impl FnMut() -> Result<()>,
    ) -> Result<()> {
        for source in files {
            for entry in walkdir::WalkDir::new(source).follow_links(false) {
                check_cancellation()?;
                let entry = entry?;
                if entry.file_type().is_symlink() {
                    return Err(anyhow!(
                        "Symbolic links are not accepted in encrypted archive sources"
                    ));
                }
            }
        }
        Ok(())
    }

    fn create_tar_archive(
        files: &[PathBuf],
        base_dir: Option<&Path>,
        output: &Path,
        check_cancellation: &mut impl FnMut() -> Result<()>,
    ) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .with_context(|| format!("创建临时 TAR 失败: {}", output.display()))?;
        let mut archive = Builder::new(file);

        for file_path in files {
            check_cancellation()?;
            if file_path.is_file() {
                let archive_path = if let Some(base) = base_dir {
                    file_path.strip_prefix(base).unwrap_or(file_path)
                } else {
                    file_path.file_name().map(Path::new).unwrap_or(file_path)
                };
                archive
                    .append_path_with_name(file_path, archive_path)
                    .with_context(|| format!("添加文件失败: {:?}", file_path))?;
            } else if file_path.is_dir() {
                let archive_path = if let Some(base) = base_dir {
                    file_path.strip_prefix(base).unwrap_or(file_path)
                } else {
                    file_path.as_path()
                };
                archive
                    .append_dir_all(archive_path, file_path)
                    .with_context(|| format!("添加目录失败: {:?}", file_path))?;
            }
        }

        archive.finish().context("完成 TAR 归档失败")?;
        check_cancellation()?;
        let file = archive.into_inner().context("关闭 TAR 归档失败")?;
        file.sync_all()?;
        Ok(())
    }

    fn decrypt_legacy_data(
        archive_path: &Path,
        password: &str,
        check_cancellation: &mut impl FnMut() -> Result<()>,
    ) -> Result<Zeroizing<Vec<u8>>> {
        check_cancellation()?;
        if archive_path.metadata()?.len() > MAX_LEGACY_TAR_AES_BYTES + 128 {
            return Err(anyhow!(
                "Legacy TAR.AES container exceeds the safe 512 MiB in-memory limit"
            ));
        }
        let mut file = File::open(archive_path)
            .with_context(|| format!("打开加密文件失败: {:?}", archive_path))?;
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic).context("读取文件头失败")?;
        if &magic != LEGACY_MAGIC {
            return Err(anyhow!("无效的文件格式"));
        }

        let mut salt = [0u8; SALT_SIZE];
        file.read_exact(&mut salt).context("读取盐值失败")?;
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        file.read_exact(&mut nonce_bytes)
            .context("读取 nonce 失败")?;
        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext)
            .context("读取旧版加密数据失败")?;
        check_cancellation()?;

        let key = Self::derive_legacy_key(password, &salt)?;
        let cipher = Aes256Gcm::new(&key);
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
            .map(Zeroizing::new)
            .map_err(|_| anyhow!("解密失败: 密码错误或文件已损坏"))?;
        check_cancellation()?;
        Ok(plaintext)
    }

    fn derive_legacy_key(password: &str, salt: &[u8]) -> Result<Key<Aes256Gcm>> {
        let mut key_bytes = [0u8; 32];
        let params = Params::new(65_536, 3, 1, Some(32))
            .map_err(|error| anyhow!("创建 Argon2 参数失败: {error:?}"))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut key_bytes)
            .map_err(|error| anyhow!("密钥派生失败: {error:?}"))?;
        let key = *Key::<Aes256Gcm>::from_slice(&key_bytes);
        key_bytes.zeroize();
        Ok(key)
    }

    fn extract_tar_file(
        tar_path: &Path,
        output_dir: &Path,
        check_cancellation: &mut impl FnMut() -> Result<()>,
    ) -> Result<()> {
        let file = File::open(tar_path)?;
        Self::extract_tar_reader(file, output_dir, check_cancellation)
    }

    fn extract_tar_reader(
        reader: impl Read,
        output_dir: &Path,
        check_cancellation: &mut impl FnMut() -> Result<()>,
    ) -> Result<()> {
        std::fs::create_dir_all(output_dir)
            .with_context(|| format!("创建输出目录失败: {:?}", output_dir))?;
        let mut archive = tar::Archive::new(reader);
        for entry in archive.entries().context("读取 TAR 条目失败")? {
            check_cancellation()?;
            let mut entry = entry.context("读取 TAR 条目失败")?;
            if !entry.unpack_in(output_dir).context("解压 TAR 条目失败")? {
                return Err(anyhow!("TAR 条目试图逃逸输出目录"));
            }
        }
        Ok(())
    }

    pub fn is_tar_aes(path: &Path) -> Result<bool> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 8];
        if file.read_exact(&mut magic).is_err() {
            return Ok(false);
        }
        Ok(&magic == LEGACY_MAGIC || &magic == TAR_AES_MAGIC_V2)
    }

    pub fn verify_password(path: &Path, password: &str) -> Result<bool> {
        Self::verify_password_cancellable(path, password, || Ok(()))
    }

    pub fn verify_password_cancellable(
        path: &Path,
        password: &str,
        mut check_cancellation: impl FnMut() -> Result<()>,
    ) -> Result<bool> {
        if AesStreamV2::is_kind(path, AesStreamKind::Tar)? {
            return AesStreamV2::verify_password_cancellable(
                path,
                password,
                AesStreamKind::Tar,
                check_cancellation,
            );
        }
        if !Self::has_legacy_magic(path)? {
            return Ok(false);
        }
        match Self::decrypt_legacy_data(path, password, &mut check_cancellation) {
            Ok(_) => Ok(true),
            Err(error) if error.to_string().contains("密码错误或文件已损坏") => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn has_legacy_magic(path: &Path) -> Result<bool> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 8];
        if file.read_exact(&mut magic).is_err() {
            return Ok(false);
        }
        Ok(&magic == LEGACY_MAGIC)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_legacy_fixture(tar_data: &[u8], output: &Path, password: &str) -> Result<()> {
        let mut salt = [0u8; SALT_SIZE];
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut salt);
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let key = TarAesEngine::derive_legacy_key(password, &salt)?;
        let cipher = Aes256Gcm::new(&key);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), tar_data)
            .map_err(|_| anyhow!("legacy fixture encryption failed"))?;
        let mut file = File::create(output)?;
        file.write_all(LEGACY_MAGIC)?;
        file.write_all(&salt)?;
        file.write_all(&nonce_bytes)?;
        file.write_all(&ciphertext)?;
        Ok(())
    }

    fn tar_bytes(source: &Path, archive_name: &Path) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        {
            let mut builder = Builder::new(&mut buffer);
            builder.append_path_with_name(source, archive_name)?;
            builder.finish()?;
        }
        Ok(buffer)
    }

    #[test]
    fn new_tar_aes_files_use_v2_and_roundtrip() -> Result<()> {
        let temp = TempDir::new()?;
        let test_file = temp.path().join("test.txt");
        fs::write(&test_file, b"Test content for AES encryption")?;
        let output = temp.path().join("test.tar.aes");
        let extract_dir = temp.path().join("extracted");

        TarAesEngine::compress_tar_aes(
            std::slice::from_ref(&test_file),
            &output,
            "test_password",
            Some(temp.path()),
        )?;
        assert_eq!(&fs::read(&output)?[..8], TAR_AES_MAGIC_V2);
        assert!(TarAesEngine::verify_password(&output, "test_password")?);
        assert!(!TarAesEngine::verify_password(&output, "wrong_password")?);
        TarAesEngine::decompress_tar_aes(&output, &extract_dir, "test_password")?;
        assert_eq!(
            fs::read_to_string(extract_dir.join("test.txt"))?,
            "Test content for AES encryption"
        );
        Ok(())
    }

    #[test]
    fn legacy_v1_tar_aes_remains_readable() -> Result<()> {
        let temp = TempDir::new()?;
        let source = temp.path().join("legacy.txt");
        let archive = temp.path().join("legacy.tar.aes");
        let output = temp.path().join("output");
        fs::write(&source, b"legacy tar data")?;
        let data = tar_bytes(&source, Path::new("legacy.txt"))?;
        write_legacy_fixture(&data, &archive, "password")?;

        assert!(TarAesEngine::verify_password(&archive, "password")?);
        TarAesEngine::decompress_tar_aes(&archive, &output, "password")?;
        assert_eq!(fs::read(output.join("legacy.txt"))?, b"legacy tar data");
        Ok(())
    }

    #[test]
    fn multiple_files_roundtrip() -> Result<()> {
        let temp = TempDir::new()?;
        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");
        fs::write(&file1, b"Content 1")?;
        fs::write(&file2, b"Content 2")?;
        let archive = temp.path().join("test.tar.aes");
        let output = temp.path().join("extracted");

        TarAesEngine::compress_tar_aes(
            &[file1, file2],
            &archive,
            "password123",
            Some(temp.path()),
        )?;
        TarAesEngine::decompress_tar_aes(&archive, &output, "password123")?;
        assert_eq!(fs::read(output.join("file1.txt"))?, b"Content 1");
        assert_eq!(fs::read(output.join("file2.txt"))?, b"Content 2");
        Ok(())
    }
}
