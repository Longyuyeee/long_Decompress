use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("文件不存在: {0}")]
    FileNotFound(String),
    #[error("压缩失败: {0}")]
    CompressionFailed(String),
    #[error("解压失败: {0}")]
    ExtractionFailed(String),
    #[error("需要输入密码才能解压")]
    PasswordRequired,
    #[error("提供的密码不正确")]
    InvalidPassword,
    #[error("密码错误")]
    PasswordError,
    #[error("不支持的加密算法或压缩方法")]
    UnsupportedEncryption,
    #[error("目标磁盘空间不足")]
    DiskFull,
    #[error("批量解压部分完成，部分文件失败")]
    PartialSuccess(Vec<String>),
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("任务已取消")]
    Cancelled,
}

fn main() {
    let err = CompressionError::PartialSuccess(vec!["test.txt".to_string()]);
    println!("Error string: {}", err);
    match err {
        CompressionError::PartialSuccess(files) => {
            println!("Failed files: {:?}", files);
            assert_eq!(files.len(), 1);
        },
        _ => panic!("Wrong variant!"),
    }
    
    println!("Verification successful!");
}
