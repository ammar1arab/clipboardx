use std::{
    fs::OpenOptions,
    io::Write,
    os::fd::AsRawFd,
    thread,
    time::Duration,
};

const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;
const SYN_REPORT: u16 = 0;
const KEY_LEFTCTRL: u16 = 29;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_V: u16 = 47;
const BUS_VIRTUAL: u16 = 6;

nix::ioctl_write_int!(ui_set_evbit, b'U', 100);
nix::ioctl_write_int!(ui_set_keybit, b'U', 101);
nix::ioctl_none!(ui_dev_create, b'U', 1);
nix::ioctl_none!(ui_dev_destroy, b'U', 2);
nix::ioctl_write_ptr!(ui_dev_setup, b'U', 3, UinputSetup);

#[repr(C)]
struct UinputSetup {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
    name: [u8; 80],
    ff_effects_max: u32,
}

#[repr(C)]
struct InputEvent {
    sec: i64,
    usec: i64,
    kind: u16,
    code: u16,
    value: i32,
}

pub fn chord(terminal: bool) -> Vec<(u16, i32)> {
    let mut keys = vec![(KEY_LEFTCTRL, 1)];
    if terminal {
        keys.push((KEY_LEFTSHIFT, 1));
    }
    keys.push((KEY_V, 1));
    keys.push((KEY_V, 0));
    if terminal {
        keys.push((KEY_LEFTSHIFT, 0));
    }
    keys.push((KEY_LEFTCTRL, 0));
    keys
}

fn event(kind: u16, code: u16, value: i32) -> InputEvent {
    InputEvent { sec: 0, usec: 0, kind, code, value }
}

fn write_event(file: &mut impl Write, kind: u16, code: u16, value: i32) -> std::io::Result<()> {
    let bytes = unsafe {
        std::slice::from_raw_parts(
            (&event(kind, code, value) as *const InputEvent).cast::<u8>(),
            std::mem::size_of::<InputEvent>(),
        )
    };
    file.write_all(bytes)
}

pub enum SendError {
    Unavailable,
    Failed(String),
}

pub fn send(terminal: bool) -> Result<(), SendError> {
    let mut device = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/uinput")
        .map_err(|_| SendError::Unavailable)?;
    let fd = device.as_raw_fd();
    unsafe {
        ui_set_evbit(fd, EV_KEY.into()).map_err(|error| SendError::Failed(error.to_string()))?;
        for key in [KEY_LEFTCTRL, KEY_LEFTSHIFT, KEY_V] {
            ui_set_keybit(fd, key.into()).map_err(|error| SendError::Failed(error.to_string()))?;
        }
        let mut name = [0u8; 80];
        name[..10].copy_from_slice(b"ClipboardX");
        ui_dev_setup(fd, &UinputSetup { bustype: BUS_VIRTUAL, vendor: 1, product: 1, version: 1, name, ff_effects_max: 0 })
            .map_err(|error| SendError::Failed(error.to_string()))?;
        ui_dev_create(fd).map_err(|error| SendError::Failed(error.to_string()))?;
    }
    let result = (|| {
        thread::sleep(Duration::from_millis(50));
        let keys = chord(terminal);
        let press_end = keys.iter().position(|(code, value)| *code == KEY_V && *value == 0).unwrap_or(keys.len());
        for (code, value) in keys.iter().take(press_end) {
            write_event(&mut device, EV_KEY, *code, *value).map_err(|error| SendError::Failed(error.to_string()))?;
        }
        write_event(&mut device, EV_SYN, SYN_REPORT, 0).map_err(|error| SendError::Failed(error.to_string()))?;
        device.flush().map_err(|error| SendError::Failed(error.to_string()))?;
        thread::sleep(Duration::from_millis(30));
        for (code, value) in keys.iter().skip(press_end) {
            write_event(&mut device, EV_KEY, *code, *value).map_err(|error| SendError::Failed(error.to_string()))?;
        }
        write_event(&mut device, EV_SYN, SYN_REPORT, 0).map_err(|error| SendError::Failed(error.to_string()))?;
        device.flush().map_err(|error| SendError::Failed(error.to_string()))?;
        thread::sleep(Duration::from_millis(30));
        Ok(())
    })();
    unsafe { let _ = ui_dev_destroy(fd); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_is_held_while_v_is_pressed_and_released() {
        let editor = chord(false);
        let v_down = editor.iter().position(|key| *key == (KEY_V, 1)).unwrap();
        let v_up = editor.iter().position(|key| *key == (KEY_V, 0)).unwrap();
        assert!(editor[..v_down].contains(&(KEY_LEFTCTRL, 1)));
        assert!(!editor[..v_up].contains(&(KEY_LEFTCTRL, 0)));
        assert_eq!(editor.last(), Some(&(KEY_LEFTCTRL, 0)));
        let terminal = chord(true);
        assert!(terminal.contains(&(KEY_LEFTSHIFT, 1)));
        assert_eq!(terminal.iter().filter(|key| key.0 == KEY_LEFTSHIFT).count(), 2);
    }

    #[test]
    fn input_event_matches_the_kernel_layout() {
        assert_eq!(std::mem::size_of::<InputEvent>(), 24);
        assert_eq!(std::mem::size_of::<UinputSetup>(), 92);
    }
}
