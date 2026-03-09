use tauri::AppHandle;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};
use lazy_static::lazy_static;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAssociation {
    pub extension: String,
    pub description: String,
    pub icon_path: Option<String>,
}

#[derive(Clone)]
pub struct FileAssociationManager {
    associations: Arc<RwLock<Vec<FileAssociation>>>,
}

impl FileAssociationManager {
    pub fn new() -> Self {
        Self {
            associations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, association: FileAssociation) -> Result<()> {
        let mut associations = self.associations.write().await;
        associations.push(association);
        Ok(())
    }
}

pub struct GlobalFileAssociationManager {
    manager: Arc<RwLock<Option<FileAssociationManager>>>,
}

impl GlobalFileAssociationManager {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(Some(FileAssociationManager::new()))),
        }
    }

    pub async fn get(&self) -> Result<Arc<FileAssociationManager>, String> {
        let manager_guard = self.manager.read().await;
        manager_guard
            .as_ref()
            .map(|m| Arc::new(m.clone()))
            .ok_or_else(|| "文件关联管理器未初始化".to_string())
    }
}

lazy_static! {
    pub static ref FILE_ASSOCIATION_MANAGER: GlobalFileAssociationManager = GlobalFileAssociationManager::new();
}
