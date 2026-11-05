use super::{publish_status, CaptureStatus};
use crate::database::History;
use nix::poll::{poll, PollFd, PollFlags};
use std::{
    io::Read,
    os::fd::AsFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use wl_clipboard_rs::{
    paste::Seat,
    watch::{CancelHandle, ClipboardEvent, ClipboardType, Watcher},
};

const MAX_TEXT_BYTES: usize = 1024 * 1024;
const READ_DEADLINE: Duration = Duration::from_secs(2);

fn image_mime(types: &[String]) -> Option<&str> {
    [
        "image/png",
        "image/x-png",
        "image/jpeg",
        "image/jpg",
        "image/webp",
        "image/bmp",
        "image/x-bmp",
        "image/tiff",
    ]
    .into_iter()
    .find_map(|candidate| {
        types
            .iter()
            .find(|mime| mime.as_str() == candidate)
            .map(String::as_str)
    })
}

fn text_mime(types: &[String]) -> Option<&str> {
    if types
        .iter()
        .any(|mime| mime == "application/x-kde-passwordManagerHint")
    {
        return None;
    }
    [
        "text/plain;charset=utf-8",
        "text/plain;charset=UTF-8",
        "UTF8_STRING",
        "text/plain",
    ]
    .into_iter()
    .find(|candidate| types.iter().any(|mime| mime == candidate))
}

fn read_bytes(
    mut reader: impl Read + AsFd,
    stop: &AtomicBool,
    limit: usize,
    deadline: Duration,
) -> Result<Vec<u8>, &'static str> {
    let deadline = Instant::now() + deadline;
    let image = limit > MAX_TEXT_BYTES;
    let mut contents = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        if stop.load(Ordering::Acquire) {
            return Err("Capture stopped");
        }
        if Instant::now() >= deadline {
            return Err(if image {
                "Clipboard image did not finish sending. Copy again."
            } else {
                "Clipboard owner did not finish sending text. Copy again to retry."
            });
        }
        let ready = {
            let mut descriptors = [PollFd::new(reader.as_fd(), PollFlags::POLLIN)];
            poll(&mut descriptors, 100u16)
                .map_err(|_| "Could not read clipboard text. Copy again to retry.")?;
            descriptors[0].revents().unwrap_or(PollFlags::empty())
        };
        if ready.is_empty() {
            continue;
        }
        let length = reader
            .read(&mut buffer)
            .map_err(|_| "Could not read clipboard text. Copy again to retry.")?;
        if length == 0 {
            break;
        }
        if contents.len() + length > limit {
            return Err(if image {
                "Clipboard image exceeds the 64 MiB limit."
            } else {
                "Clipboard text exceeds the 1 MiB capture limit."
            });
        }
        contents.extend_from_slice(&buffer[..length]);
    }
    Ok(contents)
}

#[cfg(test)]
fn read_text(reader: impl Read + AsFd, stop: &AtomicBool) -> Result<String, &'static str> {
    String::from_utf8(read_bytes(reader, stop, MAX_TEXT_BYTES, READ_DEADLINE)?)
        .map_err(|_| "Clipboard text is not valid UTF-8.")
}

fn selection_should_copy(from_terminal: bool, text: &str, previous: &str) -> bool {
    from_terminal && !text.trim().is_empty() && text != previous
}

fn startup_primary(seen: &mut bool) -> bool {
    if *seen {
        return false;
    }
    *seen = true;
    true
}

fn promote_terminal_selection(incoming: mpsc::Receiver<String>) {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return;
    };
    let mut last = String::new();
    while let Ok(mut latest) = incoming.recv() {
        loop {
            match incoming.recv_timeout(Duration::from_millis(200)) {
                Ok(newer) => latest = newer,
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
        if !selection_should_copy(true, &latest, &last) {
            continue;
        }
        if clipboard.set_text(latest.clone()).is_ok() {
            last = latest;
        }
    }
}

fn focused_terminal(app: &AppHandle) -> bool {
    app.try_state::<crate::platform::cosmic::FocusTracker>()
        .is_some_and(|tracker| tracker.focused_terminal())
}

pub(super) fn watch(
    app: AppHandle,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<CaptureStatus>>,
    cancel: Arc<Mutex<Option<CancelHandle>>>,
    retry: mpsc::Receiver<()>,
) {
    while !stop.load(Ordering::Acquire) {
        let watcher = Watcher::new(ClipboardType::Both, Seat::Unspecified)
            .or_else(|_| Watcher::new(ClipboardType::Regular, Seat::Unspecified));
        match watcher {
            Ok(mut watcher) => {
                if let Ok(mut handle) = cancel.lock() { *handle = Some(watcher.cancel_handle()); }
                if stop.load(Ordering::Acquire) { break; }
                if retry.try_recv().is_ok() { continue; }
                publish_status(&app, &status, CaptureStatus::Running);
                let (primary_tx, primary_rx) = mpsc::channel();
                let promoter = std::thread::Builder::new()
                    .name("clipboardx-select-copy".into())
                    .spawn(move || promote_terminal_selection(primary_rx))
                    .ok();
                let mut saw_primary = false;
                loop {
                    if stop.load(Ordering::Acquire) { break; }
                    match watcher.next_event() {
                        Ok(Some(ClipboardEvent::Changed {
                            clipboard: wl_clipboard_rs::paste::ClipboardType::Primary,
                            mime_types,
                            mut offer,
                            ..
                        })) =>
                        {
                            if startup_primary(&mut saw_primary) { continue; }
                            if !focused_terminal(&app) { continue; }
                            let Some(mime) = text_mime(&mime_types) else { continue; };
                            let Ok(reader) = offer.receive(mime) else { continue; };
                            let Ok(bytes) = read_bytes(reader, &stop, MAX_TEXT_BYTES, READ_DEADLINE) else { continue; };
                            let Ok(text) = String::from_utf8(bytes) else { continue; };
                            if selection_should_copy(true, &text, "") {
                                let _ = primary_tx.send(text);
                            }
                        }
                        Ok(Some(ClipboardEvent::Changed { mime_types, mut offer, .. })) => {
                            if mime_types.iter().any(|mime| mime == "application/x-kde-passwordManagerHint") { continue; }
                            let history = app.state::<History>();
                            let capture_images = match tauri::async_runtime::block_on(history.settings()) {
                                Ok(settings) => settings.capture_images,
                                Err(_) => { publish_status(&app, &status, CaptureStatus::Error { message: "Could not read capture settings. Retry." }); continue; }
                            };
                            let image_mime = if capture_images { image_mime(&mime_types) } else { None };
                            let Some(mime) = image_mime.or_else(|| text_mime(&mime_types)) else { continue; };
                            let result = offer.receive(mime).map_err(|_| "Could not receive clipboard contents. Copy again.")
                                .and_then(|reader| read_bytes(reader, &stop, if image_mime.is_some() { 64 * 1024 * 1024 } else { MAX_TEXT_BYTES }, if image_mime.is_some() { Duration::from_secs(8) } else { READ_DEADLINE }));
                            let saved = match result {
                                Ok(bytes) if image_mime.is_some() => {
                                    if let Ok(hash) = crate::images::encoded_rgba_hash(&bytes) {
                                        if app.try_state::<super::OwnWrites>().is_some_and(|writes| writes.suppress_image(&hash)) {
                                            publish_status(&app, &status, CaptureStatus::Running);
                                            continue;
                                        }
                                    }
                                    tauri::async_runtime::block_on(history.capture_image(bytes)).map_err(|_| "Could not save image. Check image limits and storage.")
                                }
                                Ok(bytes) => match String::from_utf8(bytes) {
                                    Ok(text) if text.is_empty() => Ok(()),
                                    Ok(text) => {
                                        if app.try_state::<super::OwnWrites>().is_some_and(|writes| writes.suppress_text(&text)) {
                                            publish_status(&app, &status, CaptureStatus::Running);
                                            continue;
                                        }
                                        tauri::async_runtime::block_on(history.capture_text(&text)).map_err(|_| "Could not save clipboard text. Check storage.")
                                    }
                                    Err(_) => Err("Clipboard text is not valid UTF-8."),
                                },
                                Err(message) => Err(message),
                            };
                            match saved {
                                Ok(()) => {
                                    publish_status(&app, &status, CaptureStatus::Running);
                                    if app.emit("history-changed", ()).is_err() { publish_status(&app, &status, CaptureStatus::Error { message: "History saved. Reopen ClipboardX to refresh." }); }
                                }
                                Err(message) => publish_status(&app, &status, CaptureStatus::Error { message }),
                            }
                        }
                        Ok(Some(ClipboardEvent::Cleared { clipboard, .. })) => {
                            if clipboard == wl_clipboard_rs::paste::ClipboardType::Primary {
                                let _ = startup_primary(&mut saw_primary);
                            } else {
                                publish_status(&app, &status, CaptureStatus::Running);
                            }
                        }
                        Ok(None) => break,
                        Err(_) => {
                            publish_status(&app, &status, CaptureStatus::Unavailable { message: "Lost Wayland clipboard connection. Retrying automatically." });
                            break;
                        }
                    }
                }
                drop(primary_tx);
                if let Some(promoter) = promoter {
                    let _ = promoter.join();
                }
            }
            Err(_) => publish_status(&app, &status, CaptureStatus::Unavailable { message: "Wayland clipboard capture is unavailable. Check compositor data-control support. Retrying automatically." }),
        }
        if let Ok(mut handle) = cancel.lock() {
            *handle = None;
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        let _ = retry.recv_timeout(Duration::from_secs(3));
    }
    if let Ok(mut handle) = cancel.lock() {
        *handle = None;
    }
    publish_status(&app, &status, CaptureStatus::Stopped);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write};

    fn pipe() -> (File, File) {
        let (reader, writer) = nix::unistd::pipe().unwrap();
        (File::from(reader), File::from(writer))
    }

    #[test]
    fn copies_a_finished_terminal_selection_once() {
        assert!(selection_should_copy(true, "ls -la\n", ""));
        assert!(!selection_should_copy(false, "ls -la\n", ""));
        assert!(!selection_should_copy(true, "   \n", ""));
        assert!(!selection_should_copy(true, "ls -la\n", "ls -la\n"));
        let mut seen = false;
        assert!(startup_primary(&mut seen));
        assert!(!startup_primary(&mut seen));
    }

    #[test]
    fn prefers_utf8_and_honors_exclusion_hint() {
        assert_eq!(
            text_mime(&["text/plain".into(), "text/plain;charset=utf-8".into()]),
            Some("text/plain;charset=utf-8")
        );
        assert_eq!(
            text_mime(&[
                "text/plain".into(),
                "application/x-kde-passwordManagerHint".into()
            ]),
            None
        );
        assert_eq!(text_mime(&["image/png".into()]), None);
        assert_eq!(
            image_mime(&["image/webp".into(), "text/plain".into()]),
            Some("image/webp")
        );
        assert_eq!(image_mime(&["text/plain".into()]), None);
    }

    #[test]
    fn preserves_multilingual_text_and_rejects_invalid_utf8() {
        for bytes in ["  مرحباً English\n".as_bytes(), &[0xff]] {
            let (reader, mut writer) = pipe();
            writer.write_all(bytes).unwrap();
            drop(writer);
            let result = read_text(reader, &AtomicBool::new(false));
            if bytes == [0xff] {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap(), "  مرحباً English\n");
            }
        }
    }

    #[test]
    fn bounds_payload_and_interrupts_stalled_read() {
        let (reader, mut writer) = pipe();
        let worker = std::thread::spawn(move || {
            let _ = writer.write_all(&vec![b'x'; MAX_TEXT_BYTES + 1]);
        });
        assert!(read_text(reader, &AtomicBool::new(false))
            .unwrap_err()
            .contains("1 MiB"));
        worker.join().unwrap();
        let (reader, _writer) = pipe();
        assert_eq!(
            read_text(reader, &AtomicBool::new(true)).unwrap_err(),
            "Capture stopped"
        );
    }

    #[test]
    fn stalled_owner_times_out() {
        let (reader, _writer) = pipe();
        let start = Instant::now();
        assert!(read_text(reader, &AtomicBool::new(false))
            .unwrap_err()
            .contains("finish sending"));
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[test]
    #[ignore = "requires live Wayland data-control; replaces clipboard with synthetic fixtures"]
    fn live_wayland_text_events_and_cancellation() {
        let mut watcher = Watcher::new(ClipboardType::Regular, Seat::Unspecified)
            .expect("Wayland data-control required");
        let cancel = watcher.cancel_handle();
        let (done, wait) = mpsc::channel();
        let watchdog = std::thread::spawn(move || {
            let _ = wait.recv_timeout(Duration::from_secs(8));
            cancel.cancel();
        });
        assert!(watcher.next_event().unwrap().is_some());
        let mut writer = arboard::Clipboard::new().unwrap();
        for fixture in [
            "ClipboardX synthetic short text",
            "مرحباً ClipboardX\nsecond synthetic line",
            "https://example.test/clipboardx",
        ] {
            writer.set_text(fixture).unwrap();
            match watcher.next_event().unwrap() {
                Some(ClipboardEvent::Changed {
                    mime_types,
                    mut offer,
                    ..
                }) => {
                    let mime = text_mime(&mime_types).expect("UTF-8 text MIME required");
                    let received =
                        read_text(offer.receive(mime).unwrap(), &AtomicBool::new(false)).unwrap();
                    assert!(
                        received == fixture,
                        "Synthetic clipboard text did not round-trip"
                    );
                }
                _ => panic!("Expected synthetic text selection event"),
            }
        }
        done.send(()).unwrap();
        watchdog.join().unwrap();
        assert!(watcher.next_event().unwrap().is_none());
    }
}
