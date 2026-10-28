//! Virtual keyboard through /dev/uinput: lets keys from the phone or the TV remote reach a full-screen
//! app (the browser) as ordinary key presses. The device is created only when an app starts, after the
//! shell has opened the real input devices, so the shell never reads back its own events.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use tvos_core::input::RemoteKey;

const UI_SET_EVBIT: u32 = 0x4004_5564;
const UI_SET_KEYBIT: u32 = 0x4004_5565;
const UI_DEV_CREATE: u32 = 0x5501;

const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;

/// Linux key code for each remote key (None = not forwarded)
fn code(key: RemoteKey) -> Option<u16> {
    Some(match key {
        RemoteKey::Up => 103,
        RemoteKey::Down => 108,
        RemoteKey::Left => 105,
        RemoteKey::Right => 106,
        RemoteKey::Select => 28,    // Enter
        RemoteKey::Back => 1,       // Escape: "back" in web TV apps
        RemoteKey::PlayPause => 164, // KEY_PLAYPAUSE
        _ => return None,
    })
}

const ALL_CODES: [u16; 7] = [103, 108, 105, 106, 28, 1, 164];

pub struct UInput {
    file: File,
}

impl UInput {
    pub fn create() -> std::io::Result<Self> {
        let mut file = OpenOptions::new().write(true).open("/dev/uinput").or_else(|_| OpenOptions::new().write(true).open("/dev/input/uinput"))?;
        let fd = file.as_raw_fd();
        let ioctl = |req: u32, arg: libc::c_int| unsafe { libc::ioctl(fd, req as _, arg) };
        if ioctl(UI_SET_EVBIT, EV_KEY as i32) < 0 {
            return Err(std::io::Error::last_os_error());
        }
        for c in ALL_CODES {
            ioctl(UI_SET_KEYBIT, c as i32);
        }
        // Legacy setup struct (uinput_user_dev): name[80], id{bustype,vendor,product,version}, ff_effects_max, abs arrays
        let mut dev = [0u8; 1116];
        dev[..14].copy_from_slice(b"TVos remote kb");
        dev[80..82].copy_from_slice(&3u16.to_ne_bytes()); // BUS_USB
        dev[82..84].copy_from_slice(&0x1209u16.to_ne_bytes());
        dev[84..86].copy_from_slice(&0x7605u16.to_ne_bytes());
        dev[86..88].copy_from_slice(&1u16.to_ne_bytes());
        file.write_all(&dev)?;
        if ioctl(UI_DEV_CREATE, 0) < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { file })
    }

    fn event(&mut self, typ: u16, code: u16, value: i32) {
        // struct input_event on 64-bit: timeval (16 bytes, kernel fills it in), type, code, value
        let mut ev = [0u8; 24];
        ev[16..18].copy_from_slice(&typ.to_ne_bytes());
        ev[18..20].copy_from_slice(&code.to_ne_bytes());
        ev[20..24].copy_from_slice(&value.to_ne_bytes());
        let _ = self.file.write_all(&ev);
    }

    /// Press and release a key. Returns false if the key is not forwarded.
    pub fn tap(&mut self, key: RemoteKey) -> bool {
        let Some(c) = code(key) else { return false };
        self.event(EV_KEY, c, 1);
        self.event(EV_SYN, 0, 0);
        self.event(EV_KEY, c, 0);
        self.event(EV_SYN, 0, 0);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_forwarded_key_is_registered_with_the_device() {
        for k in [RemoteKey::Up, RemoteKey::Down, RemoteKey::Left, RemoteKey::Right, RemoteKey::Select, RemoteKey::Back, RemoteKey::PlayPause] {
            assert!(ALL_CODES.contains(&code(k).unwrap()), "{k:?} would be silently dropped by the kernel");
        }
        assert!(code(RemoteKey::Home).is_none(), "Home closes the app instead of being forwarded");
    }
}
