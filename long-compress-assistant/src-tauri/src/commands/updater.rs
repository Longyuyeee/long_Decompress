//! Keep one checked response and one active updater operation per application.
//! Tauri 1's JS check API registers a new once-install listener on every check.
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, State};

#[derive(Default)]
pub struct UpdaterState {
    busy: AtomicBool,
    pending: Mutex<Option<tauri::updater::UpdateResponse<tauri::Wry>>>,
}
struct Operation<'a>(&'a AtomicBool);
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
impl UpdaterState {
    fn acquire(&self) -> Result<Operation<'_>, String> {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "更新操作正在进行，请勿重复启动。".to_owned())?;
        Ok(Operation(&self.busy))
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    should_update: bool,
    manifest: Option<Manifest>,
}
#[derive(Serialize)]
pub struct Manifest {
    version: String,
    date: String,
    body: String,
}

#[tauri::command]
pub async fn check_app_update(
    app: AppHandle,
    state: State<'_, UpdaterState>,
) -> Result<CheckResult, String> {
    let _operation = state.acquire()?;
    *state.pending.lock().map_err(|e| e.to_string())? = None;
    let builder = tauri::updater::builder(app).skip_events();
    #[cfg(feature = "desktop-e2e")]
    let builder = if let Ok(endpoint) = std::env::var("LONG_E2E_UPDATER_ENDPOINT") {
        if !endpoint.starts_with("http://127.0.0.1:") {
            return Err("E2E updater endpoint must use loopback".into());
        }
        builder.endpoints(&[endpoint])
    } else {
        builder
    };
    let response = match builder.check().await {
        Ok(response) => response,
        Err(tauri::updater::Error::UpToDate) => {
            return Ok(CheckResult {
                should_update: false,
                manifest: None,
            })
        }
        Err(e) => return Err(e.to_string()),
    };
    let should_update = response.is_update_available();
    let manifest = should_update.then(|| Manifest {
        version: response.latest_version().to_owned(),
        date: response.date().map(ToString::to_string).unwrap_or_default(),
        body: response.body().cloned().unwrap_or_default(),
    });
    if should_update {
        *state.pending.lock().map_err(|e| e.to_string())? = Some(response);
    }
    Ok(CheckResult {
        should_update,
        manifest,
    })
}

#[tauri::command]
pub async fn install_app_update(
    state: State<'_, UpdaterState>,
    expected_version: String,
) -> Result<(), String> {
    let _operation = state.acquire()?;
    let response = {
        let mut pending = state.pending.lock().map_err(|e| e.to_string())?;
        if pending.as_ref().map(|r| r.latest_version()) != Some(expected_version.as_str()) {
            return Err("更新信息已变化，请重新检查更新。".into());
        }
        pending.take().ok_or_else(|| "请先检查更新。".to_owned())?
    };
    // Consume this response exactly once. Tauri retains download verification,
    // signature checks, installer hand-off and its native phase/progress events.
    response
        .download_and_install()
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn updater_operation_is_exclusive_and_released_on_error() {
        let state = UpdaterState::default();
        let first = state.acquire().unwrap();
        assert!(state.acquire().is_err());
        drop(first);
        assert!(state.acquire().is_ok());
    }
}

// Exercise the framework's original event API against a loopback fixture only.
#[tauri::command]
pub async fn desktop_e2e_legacy_update_check(app: AppHandle) -> Result<(), String> {
    #[cfg(not(feature = "desktop-e2e"))]
    {
        let _ = app;
        return Err("Desktop E2E bridge is disabled".into());
    }
    #[cfg(feature = "desktop-e2e")]
    {
        let endpoint = std::env::var("LONG_E2E_UPDATER_ENDPOINT").map_err(|e| e.to_string())?;
        if !endpoint.starts_with("http://127.0.0.1:") {
            return Err("E2E updater endpoint must use loopback".into());
        }
        let response=tauri::updater::builder(app)
            .endpoints(&[endpoint])
            .check()
            .await
            .map_err(|e| e.to_string())?;
        if !response.is_update_available(){return Err("Legacy fixture must offer a newer version".into());}
        Ok(())
    }
}
