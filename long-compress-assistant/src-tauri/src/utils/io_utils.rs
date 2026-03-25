use std::io::{Read, Seek, SeekFrom, Result};
use std::sync::Arc;
use std::fs::File;
use std::path::Path;
use memmap2::Mmap;

/// 一个带进度回调的 Read 包装器
pub struct ProgressReader<R: Read> {
    inner: R,
    total_size: u64,
    current_pos: u64,
    on_progress: Arc<dyn Fn(u64, u64) + Send + Sync>,
}

impl<R: Read> ProgressReader<R> {
    pub fn new(inner: R, total_size: u64, on_progress: Arc<dyn Fn(u64, u64) + Send + Sync>) -> Self {
        Self {
            inner,
            total_size,
            current_pos: 0,
            on_progress,
        }
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    pub fn current_pos(&self) -> u64 {
        self.current_pos
    }
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let bytes_read = self.inner.read(buf)?;
        if bytes_read > 0 {
            self.current_pos += bytes_read as u64;
            (self.on_progress)(self.current_pos, self.total_size);
        }
        Ok(bytes_read)
    }
}

/// 智能文件读取器，针对大文件自动启用 Mmap
pub enum SmartFileReader {
    Standard(File),
    Mapped(std::io::Cursor<Mmap>),
}

impl SmartFileReader {
    /// 内存映射阈值：1GB
    const MMAP_THRESHOLD: u64 = 1024 * 1024 * 1024;

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        if size >= Self::MMAP_THRESHOLD {
            log::info!("文件大小超过 1GB，正在启用内存映射 (Mmap) 模式进行读取...");
            let mmap = unsafe { Mmap::map(&file)? };
            Ok(SmartFileReader::Mapped(std::io::Cursor::new(mmap)))
        } else {
            Ok(SmartFileReader::Standard(file))
        }
    }
}

impl Read for SmartFileReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            SmartFileReader::Standard(f) => f.read(buf),
            SmartFileReader::Mapped(c) => c.read(buf),
        }
    }
}

impl Seek for SmartFileReader {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        match self {
            SmartFileReader::Standard(f) => f.seek(pos),
            SmartFileReader::Mapped(c) => c.seek(pos),
        }
    }
}
