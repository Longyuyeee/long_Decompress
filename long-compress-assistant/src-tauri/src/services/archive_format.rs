/// Archive formats understood by the extraction router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Rar,
    AesEncrypted,
    Tar,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
    Lzma,
    Iso,
    Cab,
    Lzh,
    Arj,
    Dmg,
    Wim,
    Vhd,
    Chm,
    Deb,
    Rpm,
    SquashFs,
    Nsis,
    Msi,
    Xar,
    Cpio,
    Udf,
    Fat,
    Ntfs,
    Hfs,
    Alz,
    Arc,
    Apfs,
    Ext,
    /// Other archive or container formats handled by the 7-Zip CLI.
    Universal,
    Unknown,
}

impl ArchiveFormat {
    pub fn from_magic(header: &[u8]) -> Self {
        if header.starts_with(b"PK\x03\x04") {
            return Self::Zip;
        }
        if header.starts_with(b"7z\xBC\xAF\x27\x1C") {
            return Self::SevenZip;
        }
        if header.starts_with(b"Rar!\x1a\x07\x00") || header.starts_with(b"Rar!\x1a\x07\x01\x00") {
            return Self::Rar;
        }
        if header.starts_with(b"TARAES01")
            || header.starts_with(b"AESENC01")
            || header.starts_with(b"TARAES02")
            || header.starts_with(b"AESENC02")
        {
            return Self::AesEncrypted;
        }
        if header.starts_with(b"\x1F\x8B") {
            return Self::Gzip;
        }
        if header.starts_with(b"BZh") {
            return Self::Bzip2;
        }
        if header.starts_with(b"\xFD7zXZ\x00") {
            return Self::Xz;
        }
        if header.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
            return Self::Zstd;
        }
        if header.starts_with(b"\x5D\x00\x00") {
            return Self::Lzma;
        }
        if header.starts_with(b"MSCF") {
            return Self::Cab;
        }
        if header.len() >= 5
            && header[2] == b'-'
            && header[3] == b'l'
            && matches!(header[4], b'h' | b'z')
        {
            return Self::Lzh;
        }
        if header.starts_with(b"\x60\xEA") {
            return Self::Arj;
        }
        if header.starts_with(b"\x78\x01\x73\x0D") {
            return Self::Dmg;
        }
        if header.starts_with(b"MSWIM\x00\x00\x00") {
            return Self::Wim;
        }
        if header.starts_with(b"conectix") {
            return Self::Vhd;
        }
        if header.starts_with(b"ITSF") {
            return Self::Chm;
        }
        if header.starts_with(b"!<arch>\n") {
            return Self::Deb;
        }
        if header.starts_with(&[0xED, 0xAB, 0xEE, 0xDB]) {
            return Self::Rpm;
        }
        if header.starts_with(b"hsqs") || header.starts_with(b"sqsh") {
            return Self::SquashFs;
        }
        if header.starts_with(b"070707")
            || header.starts_with(b"070701")
            || header.starts_with(b"070702")
        {
            return Self::Cpio;
        }
        if header.len() >= 262 && &header[257..262] == b"ustar" {
            return Self::Tar;
        }

        Self::Unknown
    }

    pub fn from_extension(extension: &str) -> Self {
        match extension
            .trim_start_matches('.')
            .to_ascii_lowercase()
            .as_str()
        {
            "zip" | "zipx" | "jar" | "xpi" | "odt" | "ods" | "docx" | "xlsx" | "pptx" | "epub"
            | "ipa" | "apk" | "appx" => Self::Zip,
            "7z" => Self::SevenZip,
            "rar" => Self::Rar,
            "aes" => Self::AesEncrypted,
            "tar" | "ova" => Self::Tar,
            "gz" | "gzip" | "tgz" | "tpz" => Self::Gzip,
            "bz2" | "bzip2" | "tbz" | "tbz2" => Self::Bzip2,
            "xz" | "txz" => Self::Xz,
            "zst" | "zstd" | "tzst" => Self::Zstd,
            "lzma" => Self::Lzma,
            "iso" | "img" => Self::Iso,
            "cab" => Self::Cab,
            "lzh" | "lha" => Self::Lzh,
            "arj" => Self::Arj,
            "dmg" => Self::Dmg,
            "wim" => Self::Wim,
            "vhd" | "vhdx" => Self::Vhd,
            "chm" => Self::Chm,
            "deb" => Self::Deb,
            "rpm" => Self::Rpm,
            "sfs" | "squashfs" => Self::SquashFs,
            "nsis" => Self::Nsis,
            "msi" => Self::Msi,
            "xar" => Self::Xar,
            "cpio" => Self::Cpio,
            "udf" => Self::Udf,
            "fat" => Self::Fat,
            "ntfs" => Self::Ntfs,
            "hfs" | "hfsx" => Self::Hfs,
            "alz" => Self::Alz,
            "arc" => Self::Arc,
            "apfs" => Self::Apfs,
            "ext" | "ext2" | "ext3" | "ext4" => Self::Ext,
            "apm" | "ar" | "a" | "cramfs" | "gpt" | "mbr" | "ihex" | "qcow" | "qcow2"
            | "qcow2c" | "scap" | "uefif" | "vdi" | "vmdk" | "z" | "taz" | "swm" | "esd"
            | "ppkg" | "msp" | "msm" | "udeb" | "001" | "002" | "003" | "004" | "005" | "006"
            | "007" | "008" | "009" | "z01" | "z02" | "z03" | "z04" | "z05" | "z06" | "z07"
            | "z08" | "z09" => Self::Universal,
            _ => Self::Unknown,
        }
    }

    pub fn from_password_extension(extension: &str) -> Self {
        match Self::from_extension(extension) {
            Self::Zip => Self::Zip,
            Self::SevenZip => Self::SevenZip,
            Self::Rar => Self::Rar,
            Self::AesEncrypted => Self::AesEncrypted,
            _ => Self::Universal,
        }
    }

    pub fn supports_password(&self) -> bool {
        matches!(
            self,
            Self::Zip | Self::SevenZip | Self::Rar | Self::AesEncrypted | Self::Universal
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ArchiveFormat;

    #[test]
    fn detects_all_native_magic_signatures() {
        let cases: &[(&[u8], ArchiveFormat)] = &[
            (b"PK\x03\x04", ArchiveFormat::Zip),
            (b"7z\xBC\xAF\x27\x1C", ArchiveFormat::SevenZip),
            (b"Rar!\x1a\x07\x00", ArchiveFormat::Rar),
            (b"Rar!\x1a\x07\x01\x00", ArchiveFormat::Rar),
            (b"TARAES01payload", ArchiveFormat::AesEncrypted),
            (b"AESENC02payload", ArchiveFormat::AesEncrypted),
            (b"\x1F\x8B", ArchiveFormat::Gzip),
            (b"BZh", ArchiveFormat::Bzip2),
            (b"\xFD7zXZ\x00", ArchiveFormat::Xz),
            (&[0x28, 0xB5, 0x2F, 0xFD], ArchiveFormat::Zstd),
            (b"\x5D\x00\x00", ArchiveFormat::Lzma),
            (b"MSCF", ArchiveFormat::Cab),
            (b"00-lh", ArchiveFormat::Lzh),
            (b"\x60\xEA", ArchiveFormat::Arj),
            (b"MSWIM\x00\x00\x00", ArchiveFormat::Wim),
            (b"ITSF", ArchiveFormat::Chm),
            (b"!<arch>\n", ArchiveFormat::Deb),
            (&[0xED, 0xAB, 0xEE, 0xDB], ArchiveFormat::Rpm),
            (b"hsqs", ArchiveFormat::SquashFs),
            (b"070701", ArchiveFormat::Cpio),
        ];

        for (header, expected) in cases {
            assert_eq!(ArchiveFormat::from_magic(header), expected.clone());
        }
    }

    #[test]
    fn detects_tar_magic_at_ustar_offset() {
        let mut header = vec![0; 512];
        header[257..262].copy_from_slice(b"ustar");
        assert_eq!(ArchiveFormat::from_magic(&header), ArchiveFormat::Tar);
    }

    #[test]
    fn maps_native_and_universal_extensions() {
        assert_eq!(ArchiveFormat::from_extension(".ZIPX"), ArchiveFormat::Zip);
        assert_eq!(ArchiveFormat::from_extension("hfsx"), ArchiveFormat::Hfs);
        assert_eq!(
            ArchiveFormat::from_extension("qcow2"),
            ArchiveFormat::Universal
        );
        assert_eq!(
            ArchiveFormat::from_extension("z03"),
            ArchiveFormat::Universal
        );
        assert_eq!(
            ArchiveFormat::from_extension("unknown"),
            ArchiveFormat::Unknown
        );
    }

    #[test]
    fn password_detection_keeps_unknown_formats_on_the_universal_engine() {
        assert_eq!(
            ArchiveFormat::from_password_extension("docx"),
            ArchiveFormat::Zip
        );
        assert_eq!(
            ArchiveFormat::from_password_extension("tar"),
            ArchiveFormat::Universal
        );
        assert!(ArchiveFormat::Universal.supports_password());
        assert!(!ArchiveFormat::Tar.supports_password());
    }
}
