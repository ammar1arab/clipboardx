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

static ON_TOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn apply_on_top(app: &AppHandle, window: &tauri::WebviewWindow, on_top: bool, reopen: bool) {
    ON_TOP.store(on_top, std::sync::atomic::Ordering::SeqCst);
    let _ = window.set_always_on_top(on_top);
    #[cfg(target_os = "linux")]
    layer::apply(app, window, on_top, reopen);
    #[cfg(not(target_os = "linux"))]
    let _ = (app, reopen);
}

pub fn toggle(app: &AppHandle) -> Result<(), AppError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(AppError::not_found)?;
    let result = if window.is_visible().unwrap_or(false) {
        window.hide()
    } else {
        apply_on_top(
            app,
            &window,
            ON_TOP.load(std::sync::atomic::Ordering::SeqCst),
            false,
        );
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

#[cfg(target_os = "linux")]
mod layer {
    use super::*;
    use gtk::glib::object::ObjectType;
    use gtk::prelude::WidgetExt;
    use std::ffi::c_void;
    use std::sync::OnceLock;

    struct Api {
        init: unsafe extern "C" fn(*mut c_void),
        is_layer: unsafe extern "C" fn(*mut c_void) -> i32,
        set_layer: unsafe extern "C" fn(*mut c_void, i32),
        set_keyboard: unsafe extern "C" fn(*mut c_void, i32),
        set_zone: unsafe extern "C" fn(*mut c_void, i32),
        set_anchor: unsafe extern "C" fn(*mut c_void, i32, i32),
        set_margin: unsafe extern "C" fn(*mut c_void, i32, i32),
    }

    fn api() -> Option<&'static Api> {
        static API: OnceLock<Option<Api>> = OnceLock::new();
        API.get_or_init(|| unsafe { load() }).as_ref()
    }

    unsafe fn load() -> Option<Api> {
        let lib = Box::leak(Box::new(
            libloading::Library::new("libgtk-layer-shell.so.0").ok()?,
        ));
        unsafe fn sym<T: Copy>(lib: &'static libloading::Library, name: &[u8]) -> Option<T> {
            let symbol: libloading::Symbol<T> = lib.get(name).ok()?;
            Some(*symbol)
        }
        Some(Api {
            init: sym(lib, b"gtk_layer_init_for_window\0")?,
            is_layer: sym(lib, b"gtk_layer_is_layer_window\0")?,
            set_layer: sym(lib, b"gtk_layer_set_layer\0")?,
            set_keyboard: sym(lib, b"gtk_layer_set_keyboard_mode\0")?,
            set_zone: sym(lib, b"gtk_layer_set_exclusive_zone\0")?,
            set_anchor: sym(lib, b"gtk_layer_set_anchor\0")?,
            set_margin: sym(lib, b"gtk_layer_set_margin\0")?,
        })
    }

    fn configure(api: &Api, raw: *mut c_void) {
        const TOP: i32 = 2;
        const ON_DEMAND: i32 = 2;
        const LEFT: i32 = 0;
        const TOP_EDGE: i32 = 2;
        unsafe {
            (api.set_layer)(raw, TOP);
            (api.set_keyboard)(raw, ON_DEMAND);
            (api.set_zone)(raw, 0);
        }
        let x = std::env::var("CLIPBOARDX_LAYER_X").ok().and_then(|v| v.parse().ok());
        let y = std::env::var("CLIPBOARDX_LAYER_Y").ok().and_then(|v| v.parse().ok());
        if let (Some(x), Some(y)) = (x, y) {
            unsafe {
                (api.set_anchor)(raw, LEFT, 1);
                (api.set_anchor)(raw, TOP_EDGE, 1);
                (api.set_margin)(raw, LEFT, x);
                (api.set_margin)(raw, TOP_EDGE, y);
            }
        }
    }

    pub fn apply(app: &AppHandle, window: &tauri::WebviewWindow, on_top: bool, reopen: bool) {
        let Ok(gtk_window) = window.gtk_window() else {
            return;
        };
        let Some(api) = api() else {
            return;
        };
        let raw = ObjectType::as_ptr(&gtk_window) as *mut c_void;
        let layered = unsafe { (api.is_layer)(raw) } != 0;
        if on_top && layered {
            configure(api, raw);
            return;
        }
        if on_top && !gtk_window.is_realized() {
            unsafe { (api.init)(raw) };
            if unsafe { (api.is_layer)(raw) } != 0 {
                configure(api, raw);
            }
            return;
        }
        if reopen && on_top != layered {
            reopen_process(app, window);
        }
    }

    fn reopen_process(app: &AppHandle, window: &tauri::WebviewWindow) {
        use std::sync::atomic::{AtomicBool, Ordering};
        static STARTED: AtomicBool = AtomicBool::new(false);
        if STARTED.swap(true, Ordering::SeqCst) {
            return;
        }
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let mut command = std::process::Command::new("sh");
        command
            .arg("-c")
            .arg("sleep 0.5; exec \"$0\"")
            .arg(exe)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Ok(position) = window.outer_position() {
            let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
            let x = (position.x as f64 / scale).round().max(0.0) as i32;
            let y = (position.y as f64 / scale).round().max(0.0) as i32;
            command.env("CLIPBOARDX_LAYER_X", x.to_string());
            command.env("CLIPBOARDX_LAYER_Y", y.to_string());
        }
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        if command.spawn().is_ok() {
            app.exit(0);
        }
    }
}
