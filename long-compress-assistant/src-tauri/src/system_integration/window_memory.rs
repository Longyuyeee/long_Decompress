//! Main-window geometry is independent of task drafts and user preferences.
//! Store logical normal bounds; never persist a minimized rectangle or visibility.
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};
use tauri::{Manager, Window, WindowEvent};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct SavedWindow {
    schema: u8,
    monitor: Option<String>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    maximized: bool,
}

impl SavedWindow {
    fn valid(&self) -> bool {
        self.schema == 1
            && [self.x, self.y, self.width, self.height]
                .iter()
                .all(|v| v.is_finite())
            && self.width >= 100.0
            && self.height >= 100.0
            && self.width <= 100_000.0
            && self.height <= 100_000.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

// Work-area coordinates are intentional: WINDOWPLACEMENT must not be restored
// with SetWindowPos (which takes screen coordinates and drifts with top taskbars).
fn fit(saved: &SavedWindow, origin: (i32, i32), work_size: (i32, i32), scale: f64) -> Bounds {
    let width = (saved.width.max(760.0) * scale)
        .round()
        .min(work_size.0 as f64)
        .max(1.0) as i32;
    let height = (saved.height.max(520.0) * scale)
        .round()
        .min(work_size.1 as f64)
        .max(1.0) as i32;
    let dx = (saved.x * scale)
        .round()
        .clamp(0.0, (work_size.0 - width).max(0) as f64) as i32;
    let dy = (saved.y * scale)
        .round()
        .clamp(0.0, (work_size.1 - height).max(0) as f64) as i32;
    Bounds {
        x: origin.0 + dx,
        y: origin.1 + dy,
        width,
        height,
    }
}

struct Record {
    current: Option<SavedWindow>,
    written: Option<SavedWindow>,
}

pub struct WindowMemory {
    path: PathBuf,
    record: Arc<Mutex<Record>>,
    ready: AtomicBool,
    notify: mpsc::SyncSender<()>,
}

fn write_latest(path: &std::path::Path, record: &Mutex<Record>) -> Result<(), String> {
    let mut record = record.lock().map_err(|e| e.to_string())?;
    if record.current == record.written {
        return Ok(());
    }
    if let Some(saved) = record.current.as_ref() {
        let parent = path.parent().ok_or("window state has no parent")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        serde_json::to_writer(temp.as_file_mut(), saved).map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(path).map_err(|e| e.to_string())?;
        record.written = record.current.clone();
    }
    Ok(())
}

impl WindowMemory {
    pub fn new(data_dir: PathBuf) -> Self {
        let path = data_dir.join("window-state.json");
        let saved = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<SavedWindow>(&bytes).ok())
            .filter(SavedWindow::valid);
        let record = Arc::new(Mutex::new(Record {
            current: saved.clone(),
            written: saved,
        }));
        let (notify, receiver) = mpsc::sync_channel(1);
        let worker_record = record.clone();
        let worker_path = path.clone();
        std::thread::spawn(move || {
            while receiver.recv().is_ok() {
                // Coalesce drag/resize bursts without disk writes on the UI thread.
                while receiver.recv_timeout(Duration::from_millis(300)).is_ok() {}
                if let Err(error) = write_latest(&worker_path, &worker_record) {
                    eprintln!("Failed to save window state: {error}");
                }
            }
        });
        Self {
            path,
            record,
            ready: AtomicBool::new(false),
            notify,
        }
    }

    pub fn restore(&self, window: &Window) {
        let saved = self.record.lock().ok().and_then(|r| r.current.clone());
        if let Some(saved) = saved {
            if let Err(error) = restore_native(window, &saved) {
                eprintln!("Failed to restore window state; keeping default bounds: {error}");
            }
        }
        self.ready.store(true, Ordering::SeqCst);
    }

    fn capture(&self, window: &Window) {
        if window.label() != "main" || !self.ready.load(Ordering::SeqCst) {
            return;
        }
        let previous = self.record.lock().ok().and_then(|r| r.current.clone());
        if let Ok(saved) = capture_native(window, previous.as_ref()) {
            if saved.valid() {
                if let Ok(mut record) = self.record.lock() {
                    record.current = Some(saved);
                }
                let _ = self.notify.try_send(());
            }
        }
    }

    pub fn save(&self, window: &Window) {
        self.capture(window);
        if let Err(error) = write_latest(&self.path, &self.record) {
            eprintln!("Failed to flush window state: {error}");
        }
    }

    pub fn on_event(&self, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::Moved(_)
            | WindowEvent::Resized(_)
            | WindowEvent::ScaleFactorChanged { .. } => self.capture(window),
            WindowEvent::Focused(false) | WindowEvent::CloseRequested { .. } => self.save(window),
            _ => {}
        }
    }
}

pub fn save_main(app: &tauri::AppHandle) {
    if let (Some(window), Some(memory)) = (app.get_window("main"), app.try_state::<WindowMemory>())
    {
        memory.save(&window);
    }
}

#[cfg(target_os = "windows")]
fn work_area(monitor: &tauri::Monitor) -> Result<(i32, i32), String> {
    use windows_sys::Win32::{
        Foundation::POINT,
        Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    };
    let pos = monitor.position();
    let point = POINT {
        x: pos.x + 1,
        y: pos.y + 1,
    };
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST), &mut info) } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok((
        info.rcWork.right - info.rcWork.left,
        info.rcWork.bottom - info.rcWork.top,
    ))
}

#[cfg(target_os = "windows")]
fn capture_native(window: &Window, previous: Option<&SavedWindow>) -> Result<SavedWindow, String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowPlacement, SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, WINDOWPLACEMENT,
    };
    let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
    placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
    if unsafe {
        GetWindowPlacement(
            window.hwnd().map_err(|e| e.to_string())?.0 as _,
            &mut placement,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // GetWindowPlacement flags are documented as zero. Retain the last known
    // non-minimized state instead of inferring maximization from those flags.
    if placement.showCmd == SW_SHOWMINIMIZED as u32 {
        return previous
            .cloned()
            .ok_or_else(|| "no normal bounds before minimizing".into());
    }
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("no current monitor")?;
    let scale = monitor.scale_factor();
    let rect = placement.rcNormalPosition;
    Ok(SavedWindow {
        schema: 1,
        monitor: monitor.name().cloned(),
        x: (rect.left as f64 - monitor.position().x as f64) / scale,
        y: (rect.top as f64 - monitor.position().y as f64) / scale,
        width: (rect.right as f64 - rect.left as f64) / scale,
        height: (rect.bottom as f64 - rect.top as f64) / scale,
        maximized: placement.showCmd == SW_SHOWMAXIMIZED as u32,
    })
}

#[cfg(target_os = "windows")]
fn restore_native(window: &Window, saved: &SavedWindow) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{SetWindowPlacement, SW_HIDE, WINDOWPLACEMENT},
    };
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let monitor = monitors
        .iter()
        .find(|m| saved.monitor.is_some() && m.name() == saved.monitor.as_ref())
        .cloned()
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or("no available monitor")?;
    let work = work_area(&monitor)?;
    let scale = monitor.scale_factor();
    window
        .set_min_size(Some(tauri::LogicalSize::new(
            760.0_f64.min(work.0 as f64 / scale),
            520.0_f64.min(work.1 as f64 / scale),
        )))
        .map_err(|e| e.to_string())?;
    let bounds = fit(
        saved,
        (monitor.position().x, monitor.position().y),
        work,
        scale,
    );
    let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
    placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
    placement.showCmd = SW_HIDE as u32;
    placement.rcNormalPosition = RECT {
        left: bounds.x,
        top: bounds.y,
        right: bounds.x + bounds.width,
        bottom: bounds.y + bounds.height,
    };
    if unsafe { SetWindowPlacement(window.hwnd().map_err(|e| e.to_string())?.0 as _, &placement) }
        == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    if saved.maximized {
        window.maximize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// The distributed desktop currently targets Windows. Keep other development
// platforms functional without claiming native placement parity.
#[cfg(not(target_os = "windows"))]
fn capture_native(window: &Window, previous: Option<&SavedWindow>) -> Result<SavedWindow, String> {
    if window.is_minimized().unwrap_or(false) || window.is_maximized().unwrap_or(false) {
        let mut saved = previous.cloned().ok_or("no normal bounds")?;
        if !window.is_minimized().unwrap_or(false) {
            saved.maximized = true;
        }
        return Ok(saved);
    }
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("no monitor")?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = monitor.scale_factor();
    Ok(SavedWindow {
        schema: 1,
        monitor: monitor.name().cloned(),
        x: (pos.x as f64 - monitor.position().x as f64) / scale,
        y: (pos.y as f64 - monitor.position().y as f64) / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
        maximized: false,
    })
}

#[cfg(not(target_os = "windows"))]
fn restore_native(window: &Window, saved: &SavedWindow) -> Result<(), String> {
    window
        .set_size(tauri::LogicalSize::new(
            saved.width.max(760.0),
            saved.height.max(520.0),
        ))
        .map_err(|e| e.to_string())?;
    window.center().map_err(|e| e.to_string())?;
    if saved.maximized {
        window.maximize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn saved() -> SavedWindow {
        SavedWindow {
            schema: 1,
            monitor: Some("5K".into()),
            x: 80.0,
            y: 60.0,
            width: 1500.0,
            height: 950.0,
            maximized: true,
        }
    }

    #[test]
    fn window_memory_fits_scaled_negative_and_disconnected_monitor_work_areas() {
        let s = saved();
        assert_eq!(
            fit(&s, (0, 0), (5120, 2800), 2.0),
            Bounds {
                x: 160,
                y: 120,
                width: 3000,
                height: 1900
            }
        );
        assert_eq!(
            fit(&s, (-2560, 0), (2560, 1400), 1.5),
            Bounds {
                x: -2440,
                y: 0,
                width: 2250,
                height: 1400
            }
        );
        assert_eq!(
            fit(&s, (0, 0), (1280, 680), 1.0),
            Bounds {
                x: 0,
                y: 0,
                width: 1280,
                height: 680
            }
        );
        let mut s = s;
        s.x = -50000.0;
        s.y = 50000.0;
        s.width = 500.0;
        s.height = 300.0;
        assert_eq!(
            fit(&s, (0, 0), (1000, 720), 1.0),
            Bounds {
                x: 0,
                y: 200,
                width: 760,
                height: 520
            }
        );
    }

    #[test]
    fn window_memory_recovers_invalid_records_and_atomically_replaces_latest_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("window-state.json");
        std::fs::write(&path, b"{unfinished").unwrap();
        let memory = WindowMemory::new(dir.path().to_owned());
        assert!(memory.record.lock().unwrap().current.is_none());
        memory.record.lock().unwrap().current = Some(saved());
        write_latest(&path, &memory.record).unwrap();
        let mut s = saved();
        s.width = 1800.0;
        s.maximized = false;
        memory.record.lock().unwrap().current = Some(s.clone());
        write_latest(&path, &memory.record).unwrap();
        assert_eq!(
            serde_json::from_slice::<SavedWindow>(&std::fs::read(&path).unwrap()).unwrap(),
            s
        );
        for width in [0.0, -1.0, f64::NAN, f64::INFINITY, 100001.0] {
            s.width = width;
            assert!(!s.valid());
        }
        s = saved();
        s.schema = 2;
        assert!(!s.valid());
    }
}
