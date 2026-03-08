use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sysinfo::System;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub host_name: Option<String>,
    pub cpu_count: usize,
    pub cpu_usage: f32,
    pub total_memory: u64,
    pub used_memory: u64,
    pub free_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    pub free_swap: u64,
    pub disks: Vec<DiskInfo>,
    pub network_interfaces: Vec<NetworkInterfaceInfo>,
    pub uptime: u64,
    pub load_average: LoadAverage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total_space: u64,
    pub available_space: u64,
    pub used_space: u64,
    pub is_removable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub mac_address: String,
    pub received: u64,
    pub transmitted: u64,
    pub packets_received: u64,
    pub packets_transmitted: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadAverage {
    pub one_minute: f64,
    pub five_minutes: f64,
    pub fifteen_minutes: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub status: String,
    pub start_time: u64,
    pub run_time: u64,
    pub command: Vec<String>,
}

pub struct SystemService {
    system: System,
}

impl SystemService {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self { system }
    }

    /// 刷新系统信息
    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }

    /// 获取系统信息
    pub fn get_system_info(&mut self) -> Result<SystemInfo> {
        self.refresh();

        // 等待一小段时间获取准确的CPU使用率
        std::thread::sleep(Duration::from_millis(500));
        self.system.refresh_cpu();

        let disks = self.get_disks_info();
        let network_interfaces = self.get_network_interfaces_info();

        Ok(SystemInfo {
            os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version(),
            kernel_version: System::kernel_version(),
            host_name: System::host_name(),
            cpu_count: self.system.cpus().len(),
            cpu_usage: self.get_cpu_usage(),
            total_memory: self.system.total_memory(),
            used_memory: self.system.used_memory(),
            free_memory: self.system.free_memory(),
            total_swap: self.system.total_swap(),
            used_swap: self.system.used_swap(),
            free_swap: self.system.free_swap(),
            disks,
            network_interfaces,
            uptime: self.system.uptime(),
            load_average: self.get_load_average(),
        })
    }

    /// 获取CPU使用率
    fn get_cpu_usage(&self) -> f32 {
        let cpus = self.system.cpus();
        if cpus.is_empty() {
            return 0.0;
        }

        let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
        total_usage / cpus.len() as f32
    }

    /// 获取磁盘信息
    fn get_disks_info(&self) -> Vec<DiskInfo> {
        self.system.disks()
            .iter()
            .map(|disk| {
                DiskInfo {
                    name: disk.name().to_string_lossy().to_string(),
                    mount_point: disk.mount_point().to_string_lossy().to_string(),
                    file_system: String::from_utf8_lossy(disk.file_system()).to_string(),
                    total_space: disk.total_space(),
                    available_space: disk.available_space(),
                    used_space: disk.total_space() - disk.available_space(),
                    is_removable: disk.is_removable(),
                }
            })
            .collect()
    }

    /// 获取网络接口信息
    fn get_network_interfaces_info(&self) -> Vec<NetworkInterfaceInfo> {
        self.system.networks()
            .iter()
            .map(|(name, data)| {
                NetworkInterfaceInfo {
                    name: name.to_string(),
                    mac_address: data.mac_address().to_string(),
                    received: data.received(),
                    transmitted: data.transmitted(),
                    packets_received: data.packets_received(),
                    packets_transmitted: data.packets_transmitted(),
                }
            })
            .collect()
    }

    /// 获取负载平均值
    fn get_load_average(&self) -> LoadAverage {
        let load_avg = self.system.load_average();

        LoadAverage {
            one_minute: load_avg.one,
            five_minutes: load_avg.five,
            fifteen_minutes: load_avg.fifteen,
        }
    }

    /// 获取进程列表
    pub fn get_processes(&mut self) -> Vec<ProcessInfo> {
        self.system.refresh_processes();

        self.system.processes()
            .iter()
            .map(|(pid, process)| {
                ProcessInfo {
                    pid: pid.as_u32() as i32,
                    name: process.name().to_string(),
                    cpu_usage: process.cpu_usage(),
                    memory_usage: process.memory(),
                    status: format!("{:?}", process.status()),
                    start_time: process.start_time(),
                    run_time: process.run_time(),
                    command: process.cmd().to_vec(),
                }
            })
            .collect()
    }

    /// 根据PID获取进程信息
    pub fn get_process_by_pid(&mut self, pid: i32) -> Option<ProcessInfo> {
        self.system.refresh_processes();

        let sys_pid = sysinfo::Pid::from(pid as u32);
        self.system.process(sys_pid).map(|process| {
            ProcessInfo {
                pid,
                name: process.name().to_string(),
                cpu_usage: process.cpu_usage(),
                memory_usage: process.memory(),
                status: format!("{:?}", process.status()),
                start_time: process.start_time(),
                run_time: process.run_time(),
                command: process.cmd().to_vec(),
            }
        })
    }

    /// 根据名称搜索进程
    pub fn search_processes_by_name(&mut self, name: &str) -> Vec<ProcessInfo> {
        self.system.refresh_processes();

        let name_lower = name.to_lowercase();

        self.system.processes()
            .iter()
            .filter(|(_, process)| process.name().to_lowercase().contains(&name_lower))
            .map(|(pid, process)| {
                ProcessInfo {
                    pid: pid.as_u32() as i32,
                    name: process.name().to_string(),
                    cpu_usage: process.cpu_usage(),
                    memory_usage: process.memory(),
                    status: format!("{:?}", process.status()),
                    start_time: process.start_time(),
                    run_time: process.run_time(),
                    command: process.cmd().to_vec(),
                }
            })
            .collect()
    }

    /// 终止进程
    pub fn kill_process(&mut self, pid: i32) -> Result<bool> {
        self.system.refresh_processes();

        let sys_pid = sysinfo::Pid::from(pid as u32);

        if let Some(process) = self.system.process(sys_pid) {
            Ok(process.kill())
        } else {
            Err(anyhow::anyhow!("进程不存在: {}", pid))
        }
    }

    /// 获取系统温度信息（如果可用）
    pub fn get_temperatures(&mut self) -> Vec<(String, f32)> {
        self.system.refresh_components();

        self.system.components()
            .iter()
            .map(|component| {
                (component.label().to_string(), component.temperature())
            })
            .collect()
    }

    /// 获取系统风扇速度（如果可用）
    pub fn get_fan_speeds(&mut self) -> Vec<(String, u32)> {
        self.system.refresh_components();

        self.system.components()
            .iter()
            .filter_map(|component| {
                component.fan_speed().map(|speed| (component.label().to_string(), speed))
            })
            .collect()
    }

    /// 获取系统电池信息（如果可用）
    pub fn get_battery_info(&mut self) -> Option<BatteryInfo> {
        self.system.refresh_components();

        // sysinfo目前对电池支持有限，这里返回None
        // 在实际应用中，可以使用其他库如battery
        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub state: BatteryState,
    pub percentage: f32,
    pub time_to_full: Option<u64>,
    pub time_to_empty: Option<u64>,
    pub voltage: f32,
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    Empty,
    Unknown,
}