#[cfg(target_os = "linux")]
pub mod cosmic;
#[cfg(target_os = "linux")]
mod paste_keys;
use crate::error::AppError;
use tauri::{AppHandle, Emitter, Manager};

pub struct WindowFrame(std::sync::Mutex<(u32, u32)>);

impl WindowFrame {
    pub fn new(width: u32, height: u32) -> Self {
        Self(std::sync::Mutex::new((width, height)))
    }

    pub fn get(&self) -> (u32, u32) {
        *self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set(&self, width: u32, height: u32) {
        *self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = (width, height);
    }
}

pub fn apply_saved_size(app: &AppHandle, window: &tauri::WebviewWindow) {
    let (width, height) = app.state::<WindowFrame>().get();
    let _ = window.unmaximize();
    let _ = window.set_size(tauri::LogicalSize::new(width as f64, height as f64));
}

pub fn toggle(app: &AppHandle) -> Result<(), AppError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(AppError::not_found)?;
    let result = if window.is_visible().unwrap_or(false) {
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
    use super::paste_chord;

    #[test]
    fn paste_chord_holds_ctrl_before_v() {
        let editor = paste_chord(false);
        let terminal = paste_chord(true);
        assert_eq!(&editor[..6], &["-M", "ctrl", "-P", "v", "-s", "25"]);
        assert!(editor.ends_with(&["-m", "ctrl"]));
        assert!(terminal
            .windows(4)
            .any(|part| part == ["-M", "shift", "-P", "v"]));
        assert!(terminal.contains(&"-m") && terminal.contains(&"shift"));
    }

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
fn paste_chord(terminal: bool) -> &'static [&'static str] {
    if terminal {
        &[
            "-M", "ctrl", "-M", "shift", "-P", "v", "-s", "25", "-p", "v", "-m", "shift", "-m",
            "ctrl",
        ]
    } else {
        &["-M", "ctrl", "-P", "v", "-s", "25", "-p", "v", "-m", "ctrl"]
    }
}

#[cfg(target_os = "linux")]
fn send_paste(terminal: bool) -> Result<(), AppError> {
    match paste_keys::send(terminal) {
        Ok(()) => return Ok(()),
        Err(paste_keys::SendError::Failed(message)) => {
            return Err(AppError { code: "paste_error", message });
        }
        Err(paste_keys::SendError::Unavailable) => {}
    }
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let error = || AppError {
        code: "paste_error",
        message: "Native paste failed. The item remains on the clipboard.".into(),
    };
    let mut process = Command::new("wtype")
        .args(paste_chord(terminal))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| error())?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match process.try_wait() {
            Ok(Some(status)) => return if status.success() { Ok(()) } else { Err(error()) },
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = process.kill();
                let _ = process.wait();
                return Err(error());
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn window_away(window: &tauri::WebviewWindow) -> bool {
    !window.is_visible().unwrap_or(true) && !window.is_focused().unwrap_or(true)
}

#[cfg(target_os = "linux")]
async fn keyboard_left(window: &tauri::WebviewWindow) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(700);
    let mut away_since = None;
    while std::time::Instant::now() < deadline {
        if window_away(window) {
            let since = *away_since.get_or_insert_with(std::time::Instant::now);
            if since.elapsed() >= std::time::Duration::from_millis(200) {
                return true;
            }
        } else {
            away_since = None;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    false
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
        if !keyboard_left(window).await {
            return Err(AppError {
                code: "paste_error",
                message: "The previous app did not take the keyboard. The item remains on the clipboard.".into(),
            });
        }
        tauri::async_runtime::spawn_blocking(move || send_paste(terminal))
            .await
            .map_err(|_| AppError::from("Native paste task stopped"))?
    }.await;
    if result.is_err() {
        let _ = window.show();
        let _ = window.set_focus();
    }
    result
}
