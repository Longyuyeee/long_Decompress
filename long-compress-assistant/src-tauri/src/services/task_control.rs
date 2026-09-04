use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

struct TaskControl {
    cancelled: Arc<AtomicBool>,
    paused: AtomicBool,
    gate: Mutex<()>,
    wake: Condvar,
}

impl TaskControl {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            paused: AtomicBool::new(false),
            gate: Mutex::new(()),
            wake: Condvar::new(),
        }
    }

    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
        if !paused {
            self.wake.notify_all();
        }
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
        self.wake.notify_all();
    }
}

static CONTROLS: Lazy<DashMap<String, Arc<TaskControl>>> = Lazy::new(DashMap::new);
static TASK_BY_CANCELLATION: Lazy<DashMap<usize, String>> = Lazy::new(DashMap::new);

fn cancellation_key(flag: &Arc<AtomicBool>) -> usize {
    Arc::as_ptr(flag) as usize
}

fn control_for_flag(flag: &Arc<AtomicBool>) -> Option<Arc<TaskControl>> {
    let task_id = TASK_BY_CANCELLATION.get(&cancellation_key(flag))?.clone();
    CONTROLS.get(&task_id).map(|entry| entry.clone())
}

pub(crate) fn register(task_id: &str) -> Result<Arc<AtomicBool>, String> {
    let control = Arc::new(TaskControl::new());
    match CONTROLS.entry(task_id.to_string()) {
        Entry::Vacant(entry) => {
            let flag = control.cancelled.clone();
            TASK_BY_CANCELLATION.insert(cancellation_key(&flag), task_id.to_string());
            entry.insert(control);
            Ok(flag)
        }
        Entry::Occupied(_) => Err(format!("Task is already running: {task_id}")),
    }
}

pub(crate) fn cleanup(task_id: &str) {
    if let Some((_, control)) = CONTROLS.remove(task_id) {
        TASK_BY_CANCELLATION.remove(&cancellation_key(&control.cancelled));
        control.cancel();
    }
}

pub(crate) fn cancel(task_id: &str) -> bool {
    let Some(control) = CONTROLS.get(task_id) else {
        return false;
    };
    control.cancel();
    true
}

pub(crate) fn pause(task_id: &str) -> bool {
    let Some(control) = CONTROLS.get(task_id) else {
        return false;
    };
    if control.cancelled.load(Ordering::SeqCst) {
        return false;
    }
    control.set_paused(true);
    true
}

pub(crate) fn resume(task_id: &str) -> bool {
    let Some(control) = CONTROLS.get(task_id) else {
        return false;
    };
    control.set_paused(false);
    true
}

pub(crate) fn is_paused(flag: &Arc<AtomicBool>) -> bool {
    control_for_flag(flag).is_some_and(|control| control.paused.load(Ordering::SeqCst))
}

/// Blocks native archive work at its existing cancellation checkpoints. A
/// cancellation always releases the gate, so a paused task can still stop.
pub(crate) fn wait_if_paused(flag: &Arc<AtomicBool>) {
    let Some(control) = control_for_flag(flag) else {
        return;
    };
    if !control.paused.load(Ordering::SeqCst) {
        return;
    }
    let mut guard = control
        .gate
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    while control.paused.load(Ordering::SeqCst) && !control.cancelled.load(Ordering::SeqCst) {
        guard = control
            .wake
            .wait(guard)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }
}

#[cfg(target_os = "windows")]
fn set_process_suspended(process_id: u32, suspended: bool) -> Result<(), String> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};

    #[link(name = "ntdll")]
    extern "system" {
        fn NtSuspendProcess(process: HANDLE) -> i32;
        fn NtResumeProcess(process: HANDLE) -> i32;
    }

    let process = unsafe { OpenProcess(PROCESS_SUSPEND_RESUME, 0, process_id) };
    if process.is_null() {
        return Err(format!(
            "Unable to open child process {process_id} for pause control"
        ));
    }
    let status = unsafe {
        if suspended {
            NtSuspendProcess(process)
        } else {
            NtResumeProcess(process)
        }
    };
    unsafe { CloseHandle(process) };
    if status < 0 {
        return Err(format!(
            "Windows rejected {} for child process {process_id} (NTSTATUS {status:#x})",
            if suspended { "pause" } else { "resume" },
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_process_suspended(_process_id: u32, _suspended: bool) -> Result<(), String> {
    Err("External process pause is only supported on Windows".to_string())
}

/// Synchronizes a child process with its task pause flag. `suspended` is owned
/// by the runner so NtSuspendProcess/NtResumeProcess remain balanced.
pub(crate) fn sync_child_pause(
    flag: &Arc<AtomicBool>,
    process_id: u32,
    suspended: &mut bool,
) -> Result<(), String> {
    let should_suspend = is_paused(flag) && !flag.load(Ordering::SeqCst);
    if should_suspend == *suspended {
        return Ok(());
    }
    set_process_suspended(process_id, should_suspend)?;
    *suspended = should_suspend;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn native_pause_blocks_until_resume_and_cancel_wakes_it() {
        let task_id = format!("pause-gate-{}", uuid::Uuid::new_v4());
        let flag = register(&task_id).unwrap();
        assert!(pause(&task_id));

        let worker_flag = flag.clone();
        let worker = std::thread::spawn(move || {
            wait_if_paused(&worker_flag);
            worker_flag.load(Ordering::SeqCst)
        });
        std::thread::sleep(Duration::from_millis(40));
        assert!(!worker.is_finished());
        assert!(resume(&task_id));
        assert!(!worker.join().unwrap());

        assert!(pause(&task_id));
        let worker_flag = flag.clone();
        let worker = std::thread::spawn(move || {
            wait_if_paused(&worker_flag);
            worker_flag.load(Ordering::SeqCst)
        });
        std::thread::sleep(Duration::from_millis(40));
        assert!(cancel(&task_id));
        assert!(worker.join().unwrap());
        cleanup(&task_id);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_child_process_stops_writing_while_suspended_and_resumes() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("pause-observation.txt");
        let escaped = output.to_string_lossy().replace('\'', "''");
        let script = format!(
            "$p='{escaped}'; while ($true) {{ [IO.File]::AppendAllText($p, 'x'); Start-Sleep -Milliseconds 20 }}"
        );
        let mut child = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .spawn()
            .unwrap();

        let task_id = format!("external-pause-{}", uuid::Uuid::new_v4());
        let flag = register(&task_id).unwrap();
        let mut suspended = false;
        let result = (|| -> Result<(), String> {
            let started = (0..100).any(|_| {
                let ready = std::fs::metadata(&output).is_ok_and(|metadata| metadata.len() > 0);
                if !ready {
                    std::thread::sleep(Duration::from_millis(50));
                }
                ready
            });
            if !started {
                return Err("PowerShell observation process did not start writing".to_string());
            }
            if !pause(&task_id) {
                return Err("Task control refused a running pause request".to_string());
            }
            sync_child_pause(&flag, child.id(), &mut suspended)?;
            std::thread::sleep(Duration::from_millis(100));
            let paused_size = std::fs::metadata(&output)
                .map_err(|error| error.to_string())?
                .len();
            std::thread::sleep(Duration::from_millis(200));
            if std::fs::metadata(&output)
                .map_err(|error| error.to_string())?
                .len()
                != paused_size
            {
                return Err("Suspended child continued writing".to_string());
            }
            if !resume(&task_id) {
                return Err("Task control refused resume".to_string());
            }
            sync_child_pause(&flag, child.id(), &mut suspended)?;
            let resumed = (0..100).any(|_| {
                let grew =
                    std::fs::metadata(&output).is_ok_and(|metadata| metadata.len() > paused_size);
                if !grew {
                    std::thread::sleep(Duration::from_millis(50));
                }
                grew
            });
            if !resumed {
                return Err("Resumed child did not continue writing".to_string());
            }
            Ok(())
        })();

        if suspended {
            let _ = set_process_suspended(child.id(), false);
        }
        let _ = child.kill();
        let _ = child.wait();
        cleanup(&task_id);
        result.unwrap();
    }
}
