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

pub fn apply_on_top(app: &AppHandle, window: &tauri::WebviewWindow, on_top: bool) {
    let _ = window.set_always_on_top(on_top);
    #[cfg(target_os = "linux")]
    pin_above_wayland(app, window, on_top);
}

#[cfg(target_os = "linux")]
fn pin_above_wayland(app: &AppHandle, window: &tauri::WebviewWindow, on_top: bool) {
    use gtk::glib::object::ObjectType;
    use gtk::prelude::WidgetExt;
    use std::ffi::c_void;
    let Ok(gtk_window) = window.gtk_window() else { return };
    let Some(layer) = layer_shell() else {
        return;
    };
    let raw = ObjectType::as_ptr(&gtk_window).cast::<c_void>();
    let layered = unsafe { (layer.is_layer)(raw) != 0 };
    if on_top {
        if !layered {
            let pos = window.outer_position().ok();
            if gtk_window.is_realized() {
                gtk_window.hide();
                gtk_window.unrealize();
            }
            unsafe { (layer.init)(raw) };
            unsafe {
                (layer.set_anchor)(raw, 0, 1);
                (layer.set_anchor)(raw, 2, 1);
            }
            if let Some(pos) = pos {
                unsafe {
                    (layer.set_margin)(raw, 0, pos.x.max(0));
                    (layer.set_margin)(raw, 2, pos.y.max(0));
                }
            }
        }
        unsafe {
            (layer.set_layer)(raw, 3);
            (layer.set_keyboard)(raw, 2);
            (layer.set_zone)(raw, 0);
        }
        gtk_window.show();
        let _ = window.show();
    } else if layered {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new("sh").arg("-c").arg("sleep 0.4; exec \"$0\" \"$@\"").arg(exe).spawn();
        }
        app.exit(0);
    }
}

#[cfg(target_os = "linux")]
struct LayerShell {
    init: unsafe extern "C" fn(*mut std::ffi::c_void),
    is_layer: unsafe extern "C" fn(*mut std::ffi::c_void) -> i32,
    set_layer: unsafe extern "C" fn(*mut std::ffi::c_void, i32),
    set_keyboard: unsafe extern "C" fn(*mut std::ffi::c_void, i32),
    set_zone: unsafe extern "C" fn(*mut std::ffi::c_void, i32),
    set_anchor: unsafe extern "C" fn(*mut std::ffi::c_void, i32, i32),
    set_margin: unsafe extern "C" fn(*mut std::ffi::c_void, i32, i32),
}

#[cfg(target_os = "linux")]
fn layer_shell() -> Option<&'static LayerShell> {
    use std::sync::OnceLock;
    static LAYER: OnceLock<Option<LayerShell>> = OnceLock::new();
    LAYER.get_or_init(|| {
        let lib = unsafe { libloading::Library::new("libgtk-layer-shell.so.0").ok() }?;
        let shell = unsafe {
            LayerShell {
                init: *lib.get(b"gtk_layer_init_for_window\0").ok()?,
                is_layer: *lib.get(b"gtk_layer_is_layer_window\0").ok()?,
                set_layer: *lib.get(b"gtk_layer_set_layer\0").ok()?,
                set_keyboard: *lib.get(b"gtk_layer_set_keyboard_mode\0").ok()?,
                set_zone: *lib.get(b"gtk_layer_set_exclusive_zone\0").ok()?,
                set_anchor: *lib.get(b"gtk_layer_set_anchor\0").ok()?,
                set_margin: *lib.get(b"gtk_layer_set_margin\0").ok()?,
            }
        };
        std::mem::forget(lib);
        Some(shell)
    }).as_ref()
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
