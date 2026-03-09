use sysinfo::System;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub host_name: String,
    pub cpu_count: usize,
    pub cpu_brand: String,
    pub memory_total: u64,
    pub memory_used: u64,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub mac_address: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

pub struct SystemService {
    system: System,
}

impl SystemService {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    pub fn get_system_info(&mut self) -> SystemInfo {
        self.system.refresh_all();
        
        SystemInfo {
            os_name: System::name().unwrap_or_default(),
            os_version: System::os_version().unwrap_or_default(),
            host_name: System::host_name().unwrap_or_default(),
            cpu_count: self.system.cpus().len(),
            cpu_brand: self.system.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default(),
            memory_total: self.system.total_memory(),
            memory_used: self.system.used_memory(),
            uptime: System::uptime(),
        }
    }

    pub fn get_disks(&mut self) -> Vec<DiskInfo> {
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();
        disks.iter().map(|disk| {
            DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_space: disk.total_space(),
                available_space: disk.available_space(),
                is_removable: disk.is_removable(),
            }
        }).collect()
    }

    pub fn get_networks(&mut self) -> Vec<NetworkInterfaceInfo> {
        use sysinfo::Networks;
        let networks = Networks::new_with_refreshed_list();
        networks.iter().map(|(name, data)| {
            NetworkInterfaceInfo {
                name: name.clone(),
                mac_address: data.mac_address().to_string(),
                rx_bytes: data.received(),
                tx_bytes: data.transmitted(),
            }
        }).collect()
    }
}
