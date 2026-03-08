use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库文件路径
    pub path: PathBuf,

    /// 数据库密码（可选）
    pub password: Option<String>,

    /// 连接池配置
    pub pool_config: PoolConfig,

    /// 性能配置
    pub performance_config: PerformanceConfig,

    /// 备份配置
    pub backup_config: BackupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// 最大连接数
    pub max_connections: u32,

    /// 最小连接数
    pub min_connections: u32,

    /// 连接超时时间（秒）
    pub connect_timeout: u64,

    /// 空闲连接超时时间（秒）
    pub idle_timeout: u64,

    /// 连接最大生存时间（秒）
    pub max_lifetime: u64,

    /// 获取连接超时时间（秒）
    pub acquire_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// 是否启用WAL模式
    pub wal_mode: bool,

    /// 同步模式（0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA）
    pub synchronous: u8,

    /// 缓存大小（页数）
    pub cache_size: i64,

    /// 页面大小（字节）
    pub page_size: i64,

    /// 是否启用外键约束
    pub foreign_keys: bool,

    /// 是否启用自动清理
    pub auto_vacuum: bool,

    /// 是否启用内存映射
    pub mmap_size: i64,

    /// 是否启用预写日志
    pub journal_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// 是否启用自动备份
    pub auto_backup: bool,

    /// 备份间隔（小时）
    pub backup_interval_hours: u32,

    /// 保留备份数量
    pub retain_backup_count: u32,

    /// 备份目录
    pub backup_dir: PathBuf,

    /// 是否启用压缩备份
    pub compress_backup: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let data_dir = if cfg!(debug_assertions) {
            PathBuf::from("./data")
        } else {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("long-compress-assistant")
        };

        Self {
            path: data_dir.join("database.sqlite"),
            password: None,
            pool_config: PoolConfig::default(),
            performance_config: PerformanceConfig::default(),
            backup_config: BackupConfig::default(),
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            connect_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 1800,
            acquire_timeout: 30,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            wal_mode: true,
            synchronous: 1, // NORMAL
            cache_size: -2000, // 2MB
            page_size: 4096,
            foreign_keys: true,
            auto_vacuum: true,
            mmap_size: 134217728, // 128MB
            journal_mode: "WAL".to_string(),
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        let data_dir = if cfg!(debug_assertions) {
            PathBuf::from("./data")
        } else {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("long-compress-assistant")
        };

        Self {
            auto_backup: true,
            backup_interval_hours: 24,
            retain_backup_count: 7,
            backup_dir: data_dir.join("backups"),
            compress_backup: true,
        }
    }
}

impl DatabaseConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // 从环境变量读取数据库路径
        if let Ok(path) = std::env::var("DATABASE_PATH") {
            config.path = PathBuf::from(path);
        }

        // 从环境变量读取密码
        if let Ok(password) = std::env::var("DATABASE_PASSWORD") {
            config.password = Some(password);
        }

        // 从环境变量读取连接池配置
        if let Ok(max_connections) = std::env::var("DATABASE_MAX_CONNECTIONS") {
            config.pool_config.max_connections = max_connections.parse()
                .context("解析DATABASE_MAX_CONNECTIONS失败")?;
        }

        // 从环境变量读取性能配置
        if let Ok(wal_mode) = std::env::var("DATABASE_WAL_MODE") {
            config.performance_config.wal_mode = wal_mode.parse()
                .context("解析DATABASE_WAL_MODE失败")?;
        }

        // 从环境变量读取备份配置
        if let Ok(auto_backup) = std::env::var("DATABASE_AUTO_BACKUP") {
            config.backup_config.auto_backup = auto_backup.parse()
                .context("解析DATABASE_AUTO_BACKUP失败")?;
        }

        Ok(config)
    }

    /// 从配置文件加载配置
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context("读取数据库配置文件失败")?;

        let config: Self = serde_json::from_str(&content)
            .context("解析数据库配置文件失败")?;

        Ok(config)
    }

    /// 保存配置到文件
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .context("序列化数据库配置失败")?;

        tokio::fs::write(path, content)
            .await
            .context("保存数据库配置文件失败")?;

        Ok(())
    }

    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        // 验证路径
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!("数据库路径的父目录不存在: {:?}", parent));
            }
        }

        // 验证连接池配置
        if self.pool_config.max_connections == 0 {
            return Err(anyhow::anyhow!("最大连接数必须大于0"));
        }

        if self.pool_config.min_connections > self.pool_config.max_connections {
            return Err(anyhow::anyhow!("最小连接数不能大于最大连接数"));
        }

        // 验证性能配置
        if self.performance_config.synchronous > 3 {
            return Err(anyhow::anyhow!("同步模式必须是0-3之间的值"));
        }

        if self.performance_config.page_size != 1024
            && self.performance_config.page_size != 2048
            && self.performance_config.page_size != 4096
            && self.performance_config.page_size != 8192
            && self.performance_config.page_size != 16384
            && self.performance_config.page_size != 32768
            && self.performance_config.page_size != 65536 {
            return Err(anyhow::anyhow!("页面大小必须是1024、2048、4096、8192、16384、32768或65536"));
        }

        // 验证备份配置
        if self.backup_config.backup_interval_hours == 0 {
            return Err(anyhow::anyhow!("备份间隔必须大于0小时"));
        }

        if self.backup_config.retain_backup_count == 0 {
            return Err(anyhow::anyhow!("保留备份数量必须大于0"));
        }

        Ok(())
    }

    /// 获取数据库URL
    pub fn get_database_url(&self) -> String {
        format!("sqlite:{}", self.path.display())
    }

    /// 获取连接超时时间
    pub fn get_connect_timeout(&self) -> Duration {
        Duration::from_secs(self.pool_config.connect_timeout)
    }

    /// 获取获取连接超时时间
    pub fn get_acquire_timeout(&self) -> Duration {
        Duration::from_secs(self.pool_config.acquire_timeout)
    }

    /// 获取空闲连接超时时间
    pub fn get_idle_timeout(&self) -> Duration {
        Duration::from_secs(self.pool_config.idle_timeout)
    }

    /// 获取连接最大生存时间
    pub fn get_max_lifetime(&self) -> Duration {
        Duration::from_secs(self.pool_config.max_lifetime)
    }
}