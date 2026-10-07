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

fn text_mime(types: &[String]) -> Option<&str> {
    // Honor clipboard-owner privacy hints, including ClipboardX's own excluded writes.
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

fn read_text(mut reader: impl Read + AsFd, stop: &AtomicBool) -> Result<String, &'static str> {
    let deadline = Instant::now() + READ_DEADLINE;
    let mut contents = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        if stop.load(Ordering::Acquire) {
            return Err("Capture stopped");
        }
        if Instant::now() >= deadline {
            return Err("Clipboard owner did not finish sending text. Copy again to retry.");
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
        if contents.len() + length > MAX_TEXT_BYTES {
            return Err("Clipboard text exceeds the 1 MiB capture limit.");
        }
        contents.extend_from_slice(&buffer[..length]);
    }
    String::from_utf8(contents).map_err(|_| "Clipboard text is not valid UTF-8.")
}

pub(super) fn watch(
    app: AppHandle,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<CaptureStatus>>,
    cancel: Arc<Mutex<Option<CancelHandle>>>,
    retry: mpsc::Receiver<()>,
) {
    while !stop.load(Ordering::Acquire) {
        match Watcher::new(ClipboardType::Regular, Seat::Unspecified) {
            Ok(mut watcher) => {
                if let Ok(mut handle) = cancel.lock() { *handle = Some(watcher.cancel_handle()); }
                // Close the race between stop/retry and registering the cancellation handle.
                if stop.load(Ordering::Acquire) { break; }
                if retry.try_recv().is_ok() { continue; }
                publish_status(&app, &status, CaptureStatus::Running);
                loop {
                    if stop.load(Ordering::Acquire) { break; }
                    match watcher.next_event() {
                        Ok(Some(ClipboardEvent::Changed { mime_types, mut offer, .. })) => {
                            let Some(mime) = text_mime(&mime_types) else { continue; };
                            let result = offer.receive(mime)
                                .map_err(|_| "Could not receive clipboard text. Copy again to retry.")
                                .and_then(|reader| read_text(reader, &stop));
                            match result {
                                Ok(text) if !text.is_empty() => {
                                    let history = app.state::<History>();
                                    match tauri::async_runtime::block_on(history.capture_text(&text)) {
                                        Ok(()) => {
                                            publish_status(&app, &status, CaptureStatus::Running);
                                            if app.emit("history-changed", ()).is_err() {
                                                publish_status(&app, &status, CaptureStatus::Error { message: "History saved, but the interface could not refresh. Reopen ClipboardX." });
                                            }
                                        }
                                        Err(_) => publish_status(&app, &status, CaptureStatus::Error { message: "Could not save clipboard text. Check storage and copy again." }),
                                    }
                                }
                                Ok(_) => publish_status(&app, &status, CaptureStatus::Running),
                                Err(message) => publish_status(&app, &status, CaptureStatus::Error { message }),
                            }
                        }
                        Ok(Some(ClipboardEvent::Cleared { .. })) => publish_status(&app, &status, CaptureStatus::Running),
                        Ok(None) => break,
                        Err(_) => {
                            publish_status(&app, &status, CaptureStatus::Unavailable { message: "Lost Wayland clipboard connection. Retrying automatically." });
                            break;
                        }
                    }
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
        // Discard the initial offer without reading any user's existing clipboard content.
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
                    // Never include received clipboard contents in assertion output.
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
