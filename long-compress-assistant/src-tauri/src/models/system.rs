use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub host_name: Option<String>,
    pub cpu_info: CpuInfo,
    pub memory_info: MemoryInfo,
    pub disk_info: Vec<DiskInfo>,
    pub network_info: Vec<NetworkInterfaceInfo>,
    pub system_uptime: u64,
    pub load_average: LoadAverage,
    pub battery_info: Option<BatteryInfo>,
    pub temperature_info: Vec<TemperatureInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub vendor: String,
    pub brand: String,
    pub frequency: u64,
    pub cores: usize,
    pub threads: usize,
    pub usage: f32,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_free: u64,
    pub cache: u64,
    pub buffers: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total_space: u64,
    pub available_space: u64,
    pub used_space: u64,
    pub is_removable: bool,
    pub read_speed: Option<u64>,
    pub write_speed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub mac_address: String,
    pub ip_addresses: Vec<String>,
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
    pub received_packets: u64,
    pub transmitted_packets: u64,
    pub errors_in: u64,
    pub errors_out: u64,
    pub speed: Option<u64>, // Mbps
    pub status: NetworkInterfaceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkInterfaceStatus {
    Up,
    Down,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadAverage {
    pub one_minute: f64,
    pub five_minutes: f64,
    pub fifteen_minutes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub state: BatteryState,
    pub percentage: f32,
    pub time_to_full: Option<u64>, // 秒
    pub time_to_empty: Option<u64>, // 秒
    pub voltage: f32,
    pub temperature: Option<f32>,
    pub cycle_count: Option<u32>,
    pub design_capacity: Option<u64>,
    pub full_capacity: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    Empty,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureInfo {
    pub label: String,
    pub temperature: f32,
    pub critical: Option<f32>,
    pub max: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub virtual_memory: u64,
    pub status: ProcessStatus,
    pub start_time: u64,
    pub run_time: u64,
    pub command: Vec<String>,
    pub user: Option<String>,
    pub priority: i32,
    pub threads: u32,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessStatus {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Idle,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: SystemTime,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub disk_io: DiskIoMetrics,
    pub network_io: NetworkIoMetrics,
    pub process_count: u32,
    pub thread_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIoMetrics {
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub read_ops_per_sec: u64,
    pub write_ops_per_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIoMetrics {
    pub received_bytes_per_sec: u64,
    pub transmitted_bytes_per_sec: u64,
    pub received_packets_per_sec: u64,
    pub transmitted_packets_per_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemAlert {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: SystemTime,
    pub acknowledged: bool,
    pub component: Option<String>,
    pub value: Option<f32>,
    pub threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    CpuUsage,
    MemoryUsage,
    DiskUsage,
    DiskIo,
    NetworkIo,
    Temperature,
    Battery,
    Process,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub monitoring_enabled: bool,
    pub alert_enabled: bool,
    pub update_interval: u64, // 毫秒
    pub retention_days: u32,
    pub metrics_to_collect: Vec<MetricType>,
    pub alert_thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricType {
    Cpu,
    Memory,
    Disk,
    Network,
    Process,
    Temperature,
    Battery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub cpu_usage_warning: f32,
    pub cpu_usage_critical: f32,
    pub memory_usage_warning: f32,
    pub memory_usage_critical: f32,
    pub disk_usage_warning: f32,
    pub disk_usage_critical: f32,
    pub temperature_warning: f32,
    pub temperature_critical: f32,
    pub battery_warning: f32,
    pub battery_critical: f32,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            monitoring_enabled: true,
            alert_enabled: true,
            update_interval: 5000, // 5秒
            retention_days: 30,
            metrics_to_collect: vec![
                MetricType::Cpu,
                MetricType::Memory,
                MetricType::Disk,
                MetricType::Network,
            ],
            alert_thresholds: AlertThresholds {
                cpu_usage_warning: 80.0,
                cpu_usage_critical: 95.0,
                memory_usage_warning: 85.0,
                memory_usage_critical: 95.0,
                disk_usage_warning: 85.0,
                disk_usage_critical: 95.0,
                temperature_warning: 70.0,
                temperature_critical: 85.0,
                battery_warning: 20.0,
                battery_critical: 10.0,
            },
        }
    }
}

impl SystemInfo {
    pub fn format_memory(&self) -> String {
        format!(
            "{}/{} ({:.1}%)",
            self.format_bytes(self.memory_info.used),
            self.format_bytes(self.memory_info.total),
            (self.memory_info.used as f64 / self.memory_info.total as f64) * 100.0
        )
    }

    pub fn format_disk_usage(&self, mount_point: &str) -> Option<String> {
        self.disk_info.iter()
            .find(|disk| disk.mount_point == mount_point)
            .map(|disk| {
                let usage_percent = (disk.used_space as f64 / disk.total_space as f64) * 100.0;
                format!(
                    "{}/{} ({:.1}%)",
                    self.format_bytes(disk.used_space),
                    self.format_bytes(disk.total_space),
                    usage_percent
                )
            })
    }

    pub fn format_bytes(&self, bytes: u64) -> String {
        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

        if bytes == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let bytes_f64 = bytes as f64;
        let exponent = (bytes_f64.log10() / base.log10()).floor() as i32;
        let unit_index = exponent.min(5).max(0) as usize;

        let formatted = bytes_f64 / base.powi(exponent);

        format!("{:.2} {}", formatted, UNITS[unit_index])
    }
}

impl ProcessInfo {
    pub fn format_memory(&self) -> String {
        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

        if self.memory_usage == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let bytes_f64 = self.memory_usage as f64;
        let exponent = (bytes_f64.log10() / base.log10()).floor() as i32;
        let unit_index = exponent.min(5).max(0) as usize;

        let formatted = bytes_f64 / base.powi(exponent);

        format!("{:.2} {}", formatted, UNITS[unit_index])
    }

    pub fn format_cpu_usage(&self) -> String {
        format!("{:.1}%", self.cpu_usage)
    }
}

impl SystemAlert {
    pub fn new(
        alert_type: AlertType,
        severity: AlertSeverity,
        message: String,
        component: Option<String>,
        value: Option<f32>,
        threshold: Option<f32>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            alert_type,
            severity,
            message,
            timestamp: SystemTime::now(),
            acknowledged: false,
            component,
            value,
            threshold,
        }
    }

    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }

    pub fn get_icon(&self) -> &'static str {
        match self.severity {
            AlertSeverity::Info => "ℹ️",
            AlertSeverity::Warning => "⚠️",
            AlertSeverity::Error => "❌",
            AlertSeverity::Critical => "🔥",
        }
    }
}