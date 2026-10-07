#[cfg(target_os = "linux")]
mod wayland;

use crate::error::AppError;
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread::JoinHandle;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CaptureStatus {
    Starting,
    Running,
    Unavailable { message: &'static str },
    Error { message: &'static str },
    Stopped,
}

pub struct CaptureMonitor {
    stopped: Arc<AtomicBool>,
    status: Arc<Mutex<CaptureStatus>>,
    signal: mpsc::Sender<()>,
    worker: Mutex<Option<JoinHandle<()>>>,
    #[cfg(target_os = "linux")]
    cancel: Arc<Mutex<Option<wl_clipboard_rs::watch::CancelHandle>>>,
}

pub(super) fn publish_status(app: &AppHandle, status: &Mutex<CaptureStatus>, next: CaptureStatus) {
    if let Ok(mut current) = status.lock() {
        if *current != next {
            *current = next.clone();
            // The current status remains available through get_capture_status if delivery fails.
            let _ = app.emit("capture-status", next);
        }
    }
}

impl CaptureMonitor {
    pub fn start(app: AppHandle) -> Result<Self, std::io::Error> {
        let stopped = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new(CaptureStatus::Starting));
        let (signal, receiver) = mpsc::channel();
        #[cfg(target_os = "linux")]
        let cancel = Arc::new(Mutex::new(None));
        let worker_stop = stopped.clone();
        let worker_status = status.clone();
        #[cfg(target_os = "linux")]
        let worker_cancel = cancel.clone();
        let worker = std::thread::Builder::new()
            .name("clipboardx-capture".into())
            .spawn(move || {
                #[cfg(target_os = "linux")]
                wayland::watch(app, worker_stop, worker_status, worker_cancel, receiver);
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = (worker_stop, receiver);
                    publish_status(
                        &app,
                        &worker_status,
                        CaptureStatus::Unavailable {
                            message: "Native capture is currently implemented for Linux Wayland.",
                        },
                    );
                }
            })?;
        Ok(Self {
            stopped,
            status,
            signal,
            worker: Mutex::new(Some(worker)),
            #[cfg(target_os = "linux")]
            cancel,
        })
    }

    pub fn status(&self) -> Result<CaptureStatus, AppError> {
        self.status
            .lock()
            .map(|status| status.clone())
            .map_err(|_| AppError::from("Capture status is unavailable. Restart ClipboardX."))
    }

    pub fn retry(&self) {
        let _ = self.signal.send(());
        #[cfg(target_os = "linux")]
        if let Ok(cancel) = self.cancel.lock() {
            if let Some(cancel) = cancel.as_ref() {
                cancel.cancel();
            }
        }
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        self.retry();
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

impl Drop for CaptureMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}
