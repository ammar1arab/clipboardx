use crate::error::AppError;
use cosmic_protocols::{
    toplevel_info::v1::client::{
        zcosmic_toplevel_handle_v1 as handle, zcosmic_toplevel_info_v1 as info,
    },
    toplevel_management::v1::client::zcosmic_toplevel_manager_v1 as manager,
};
use nix::poll::{poll, PollFd, PollFlags};
use std::{
    os::fd::AsFd,
    sync::{mpsc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use wayland_client::{
    delegate_noop, event_created_child,
    protocol::{wl_callback, wl_registry, wl_seat},
    Connection, Dispatch, Proxy, QueueHandle,
};

type Reply = mpsc::Sender<Result<bool, AppError>>;
enum Request {
    Restore(Reply),
    FocusedTerminal(mpsc::Sender<bool>),
    Stop,
}
struct Toplevel {
    handle: handle::ZcosmicToplevelHandleV1,
    app_id: String,
    active: bool,
}
#[derive(Default)]
struct State {
    info: Option<info::ZcosmicToplevelInfoV1>,
    manager: Option<manager::ZcosmicToplevelManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    activate_supported: bool,
    synchronized: bool,
    windows: Vec<Toplevel>,
    previous: Option<handle::ZcosmicToplevelHandleV1>,
    pending: Option<(handle::ZcosmicToplevelHandleV1, Reply, Instant)>,
}

fn has_flag(bytes: &[u8], flag: u32) -> bool {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .any(|bytes| u32::from_ne_bytes(*bytes) == flag)
}

fn failure(message: &str) -> AppError {
    AppError {
        code: "focus_error",
        message: message.into(),
    }
}
fn own_app(app_id: &str) -> bool {
    matches!(
        app_id.to_lowercase().as_str(),
        "com.ammararab.clipboardx" | "clipboardx"
    )
}
fn terminal(app_id: &str) -> bool {
    let id = app_id.to_lowercase();
    [
        "terminal",
        "cosmicterm",
        "cosmic-term",
        "alacritty",
        "kitty",
        "wezterm",
        "konsole",
        "xterm",
        "urxvt",
        "foot",
    ]
    .iter()
    .any(|name| id.contains(name))
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        queue: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            match interface.as_str() {
                "zcosmic_toplevel_info_v1" => state.info = Some(registry.bind(name, 1, queue, ())),
                "zcosmic_toplevel_manager_v1" => {
                    state.manager = Some(registry.bind(name, 1, queue, ()))
                }
                "wl_seat" if state.seat.is_none() => {
                    state.seat = Some(registry.bind(name, 1, queue, ()))
                }
                _ => (),
            }
        }
    }
}
impl Dispatch<info::ZcosmicToplevelInfoV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &info::ZcosmicToplevelInfoV1,
        event: info::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let info::Event::Toplevel { toplevel } = event {
            state.windows.push(Toplevel {
                handle: toplevel,
                app_id: String::new(),
                active: false,
            });
        }
    }
    event_created_child!(State, info::ZcosmicToplevelInfoV1, [info::EVT_TOPLEVEL_OPCODE => (handle::ZcosmicToplevelHandleV1, ())]);
}
impl Dispatch<manager::ZcosmicToplevelManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &manager::ZcosmicToplevelManagerV1,
        event: manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let manager::Event::Capabilities { capabilities } = event {
            state.activate_supported = has_flag(&capabilities, 2);
        }
    }
}
impl Dispatch<handle::ZcosmicToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &handle::ZcosmicToplevelHandleV1,
        event: handle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let handle::Event::Closed = event {
            state.windows.retain(|window| &window.handle != proxy);
            if state.previous.as_ref() == Some(proxy) {
                state.previous = None;
            }
            return;
        }
        if let Some(window) = state
            .windows
            .iter_mut()
            .find(|window| &window.handle == proxy)
        {
            match event {
                handle::Event::AppId { app_id } => window.app_id = app_id,
                handle::Event::State { state: flags } => window.active = has_flag(&flags, 2),
                _ => (),
            }
            if window.active && !window.app_id.is_empty() && !own_app(&window.app_id) {
                state.previous = Some(proxy.clone());
            }
        }
    }
}
impl Dispatch<wl_callback::WlCallback, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.synchronized = true;
    }
}
delegate_noop!(State: ignore wl_seat::WlSeat);

pub struct FocusTracker {
    requests: mpsc::Sender<Request>,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl FocusTracker {
    pub fn start() -> std::io::Result<Self> {
        let (requests, receiver) = mpsc::channel();
        let (ready, initialized) = mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("clipboardx-focus".into())
            .spawn(move || {
                let result = run(receiver, &ready);
                let _ = ready.send(result);
            })?;
        let _ = initialized.recv_timeout(Duration::from_secs(1));
        Ok(Self {
            requests,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn focused_terminal(&self) -> bool {
        let (reply, response) = mpsc::channel();
        if self.requests.send(Request::FocusedTerminal(reply)).is_err() {
            return false;
        }
        response.recv_timeout(Duration::from_millis(200)).unwrap_or(false)
    }

    pub async fn restore(&self) -> Result<bool, AppError> {
        let requests = self.requests.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let (reply, response) = mpsc::channel();
            requests
                .send(Request::Restore(reply))
                .map_err(|_| failure("COSMIC focus support is unavailable. Restart ClipboardX."))?;
            response
                .recv_timeout(Duration::from_secs(2))
                .map_err(|_| failure("Focus restoration timed out. No paste was sent."))?
        })
        .await
        .map_err(|_| failure("Focus task stopped. No paste was sent."))?
    }
    pub fn stop(&self) {
        let _ = self.requests.send(Request::Stop);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}
impl Drop for FocusTracker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(requests: mpsc::Receiver<Request>, ready: &mpsc::Sender<Result<(), ()>>) -> Result<(), ()> {
    let connection = Connection::connect_to_env().map_err(|_| ())?;
    let mut queue = connection.new_event_queue();
    let _registry = connection.display().get_registry(&queue.handle(), ());
    let mut state = State::default();
    for _ in 0..3 {
        queue.roundtrip(&mut state).map_err(|_| ())?;
    }
    if state.info.is_none()
        || state.manager.is_none()
        || state.seat.is_none()
        || !state.activate_supported
    {
        return Err(());
    }
    let _ = ready.send(Ok(()));
    loop {
        queue.dispatch_pending(&mut state).map_err(|_| ())?;
        if let Some((target, _, deadline)) = &state.pending {
            let activated = state
                .windows
                .iter()
                .find(|window| &window.handle == target)
                .filter(|window| window.active)
                .map(|window| terminal(&window.app_id));
            if let Some(is_terminal) = activated.filter(|_| state.synchronized) {
                if let Some((_, reply, _)) = state.pending.take() {
                    let _ = reply.send(Ok(is_terminal));
                }
            } else if Instant::now() >= *deadline {
                if let Some((_, reply, _)) = state.pending.take() {
                    let _ = reply.send(Err(failure(
                        "The previous app did not regain focus. No paste was sent.",
                    )));
                }
            }
        }
        while let Ok(request) = requests.try_recv() {
            match request {
                Request::Stop => return Ok(()),
                Request::FocusedTerminal(reply) => {
                    let active = state.windows.iter().any(|window| {
                        window.active && terminal(&window.app_id) && !own_app(&window.app_id)
                    });
                    let _ = reply.send(active);
                }
                Request::Restore(reply) => {
                    if state.pending.is_some() {
                        let _ = reply.send(Err(failure("A paste is already in progress.")));
                        continue;
                    }
                    match (&state.manager, &state.seat, &state.previous) {
                        (Some(manager), Some(seat), Some(target))
                            if state.activate_supported && target.is_alive() =>
                        {
                            state.synchronized = false;
                            manager.activate(target, seat);
                            connection.display().sync(&queue.handle(), ());
                            connection.flush().map_err(|_| ())?;
                            state.pending = Some((
                                target.clone(),
                                reply,
                                Instant::now() + Duration::from_millis(800),
                            ));
                        }
                        _ => {
                            let _ = reply.send(Err(failure("Focus another app before choosing an entry. COSMIC activation support is required.")));
                        }
                    }
                }
            }
        }
        connection.flush().map_err(|_| ())?;
        if let Some(read) = queue.prepare_read() {
            let mut descriptors = [PollFd::new(connection.as_fd(), PollFlags::POLLIN)];
            if poll(&mut descriptors, 40u16).map_err(|_| ())? > 0 {
                read.read().map_err(|_| ())?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paste_keys_distinguish_terminal_and_editor_targets() {
        assert!(terminal("com.system76.CosmicTerm"));
        assert!(terminal("cosmic-term"));
        assert!(terminal("org.gnome.Terminal"));
        assert!(!terminal("cursor"));
        assert!(!terminal("google-chrome"));
        assert!(own_app("com.ammararab.clipboardx"));
        assert!(!own_app("org.gnome.Terminal"));
    }
    #[test]
    #[ignore = "requires live COSMIC protocols; does not activate windows or send keys"]
    fn live_cosmic_tracking_initializes_and_stops() {
        let (send, receiver) = mpsc::channel();
        let (ready, initialized) = mpsc::channel();
        let worker = std::thread::spawn(move || run(receiver, &ready));
        assert!(initialized
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_ok());
        send.send(Request::Stop).unwrap();
        assert!(worker.join().unwrap().is_ok());
    }
}
