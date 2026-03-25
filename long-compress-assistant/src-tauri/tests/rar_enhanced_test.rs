#[cfg(test)]
mod tests {
    use long_compress_assistant::services::rar_support::RarSupportService;
    use tempfile::tempdir;
    use std::io::Write;

    #[tokio::test]
    async fn test_rar_sniffing_logic() {
        let temp_dir = tempdir().unwrap();
        let service = RarSupportService::new();

        // 1. 构造模拟 RAR4 文件
        let rar4_path = temp_dir.path().join("test_v4.rar");
        let mut f4 = std::fs::File::create(&rar4_path).unwrap();
        f4.write_all(b"Rar!\x1a\x07\x00").unwrap();
        f4.write_all(b"some data").unwrap();

        let info4 = service.detect_rar_info_v2(&rar4_path).await.unwrap();
        assert_eq!(info4.version, "4.x");
        assert!(!info4.is_rar5);

        // 2. 构造模拟 RAR5 文件 (未加密 Header)
        let rar5_path = temp_dir.path().join("test_v5.rar");
        let mut f5 = std::fs::File::create(&rar5_path).unwrap();
        f5.write_all(b"Rar!\x1a\x07\x01\x00").unwrap();
        f5.write_all(&[0x00; 8]).unwrap(); // 模拟未加密标志位

        let info5 = service.detect_rar_info_v2(&rar5_path).await.unwrap();
        assert_eq!(info5.version, "5.0+");
        assert!(info5.is_rar5);
        assert!(!info5.has_encrypted_headers);

        // 3. 构造模拟 RAR5 文件 (加密 Header)
        let rar5_enc_path = temp_dir.path().join("test_v5_enc.rar");
        let mut f5e = std::fs::File::create(&rar5_enc_path).unwrap();
        f5e.write_all(b"Rar!\x1a\x07\x01\x00").unwrap();
        f5e.write_all(&[0x80; 8]).unwrap(); // 模拟加密标志位 (第9字节最高位为1)

        let info5e = service.detect_rar_info_v2(&rar5_enc_path).await.unwrap();
        assert!(info5e.has_encrypted_headers);
    }

    #[tokio::test]
    async fn test_password_attempt_logic_compilation() {
        let temp_dir = tempdir().unwrap();
        let service = RarSupportService::new();
        
        // 此测试主要验证接口是否存在且逻辑通顺
        let dummy_path = temp_dir.path().join("dummy.rar");
        std::fs::write(&dummy_path, b"dummy content").unwrap();

        let passwords = vec!["123".to_string(), "admin".to_string()];
        
        // 运行尝试，预期由于文件损坏/格式不对而失败，但应返回 Option
        let result = service.attempt_passwords(&dummy_path, &passwords).await;
        assert!(result.is_none());
    }
}
