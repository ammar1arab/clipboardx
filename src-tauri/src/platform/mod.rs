#[cfg(target_os = "linux")]
pub mod cosmic;
use crate::error::AppError;
use tauri::{AppHandle, Emitter, Manager};

pub struct WindowFrame(std::sync::Mutex<(u32, u32)>);

impl WindowFrame {
    pub fn new(width: u32, height: u32) -> Self {
        Self(std::sync::Mutex::new((width, height)))
    }

    pub fn get(&self) -> (u32, u32) {
        *self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set(&self, width: u32, height: u32) {
        *self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = (width, height);
    }
}

pub fn apply_saved_size(app: &AppHandle, window: &tauri::WebviewWindow) {
    let (width, height) = app.state::<WindowFrame>().get();
    let _ = window.unmaximize();
    let _ = window.set_size(tauri::LogicalSize::new(width as f64, height as f64));
}

pub fn apply_on_top(_app: &AppHandle, window: &tauri::WebviewWindow, on_top: bool) {
    let _ = window.set_always_on_top(on_top);
}

pub fn toggle(app: &AppHandle) -> Result<(), AppError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(AppError::not_found)?;
    let result = if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
        window.hide()
    } else {
        apply_saved_size(app, &window);
        window
            .show()
            .and_then(|_| window.set_focus())
            .and_then(|_| window.emit("window-opened", ()))
    };
    result.map_err(|_| AppError {
        code: "window_error",
        message: "Could not activate ClipboardX. Try again.".into(),
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};
    #[derive(Default)]
    struct Protocols(Vec<String>);
    impl Dispatch<wl_registry::WlRegistry, ()> for Protocols {
        fn event(
            state: &mut Self,
            _: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let wl_registry::Event::Global { interface, .. } = event {
                state.0.push(interface);
            }
        }
    }
    #[test]
    #[ignore = "requires a live Wayland compositor"]
    fn live_compositor_protocols() {
        let connection = Connection::connect_to_env().unwrap();
        let mut queue = connection.new_event_queue();
        let _registry = connection.display().get_registry(&queue.handle(), ());
        let mut state = Protocols::default();
        queue.roundtrip(&mut state).unwrap();
        for interface in state.0 {
            if interface.contains("toplevel")
                || interface.contains("virtual_keyboard")
                || interface.contains("data_control")
            {
                println!("Supported protocol: {interface}");
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub async fn paste(
    window: &tauri::WebviewWindow,
    focus: &cosmic::FocusTracker,
) -> Result<(), AppError> {
    window.hide().map_err(|_| AppError {
        code: "window_error",
        message: "Could not hide ClipboardX.".into(),
    })?;
    let result = async {
        let terminal = focus.restore().await?;
        tauri::async_runtime::spawn_blocking(move || {
            use std::{process::{Command, Stdio}, time::{Duration, Instant}};
            let keys: &[&str] = if terminal { &["-M", "ctrl", "-M", "shift", "-k", "v", "-m", "shift", "-m", "ctrl"] } else { &["-M", "ctrl", "-k", "v", "-m", "ctrl"] };
            let error = || AppError { code: "paste_error", message: "Native paste failed. Ensure wtype is installed and virtual-keyboard support is available. The item remains on the clipboard.".into() };
            let mut process = Command::new("wtype").args(keys).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|_| error())?;
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match process.try_wait() {
                    Ok(Some(status)) => return if status.success() { Ok(()) } else { Err(error()) },
                    Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
                    _ => { let _ = process.kill(); let _ = process.wait(); return Err(error()); }
                }
            }
        }).await.map_err(|_| AppError::from("Native paste task stopped"))?
    }.await;
    if result.is_err() {
        let _ = window.show();
        let _ = window.set_focus();
    }
    result
}
