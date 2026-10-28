//! Keyboard / remote input via evdev (/dev/input/event*).

use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use tvos_core::input::RemoteKey;

/// True while a full-screen app (the browser) is running: it reads the keyboard itself, so the shell
/// only keeps listening for Home, which closes the app.
pub static APP_RUNNING: AtomicBool = AtomicBool::new(false);

fn map(code: u16) -> Option<RemoteKey> {
    Some(match code {
        103 => RemoteKey::Up,
        108 => RemoteKey::Down,
        105 => RemoteKey::Left,
        106 => RemoteKey::Right,
        28 | 96 | 352 => RemoteKey::Select, // Enter, KP Enter, KEY_OK
        1 | 14 | 158 => RemoteKey::Back,    // Esc, Backspace, KEY_BACK
        102 | 35 => RemoteKey::Home,        // Home, H
        57 | 164 | 207 => RemoteKey::PlayPause,
        115 | 13 | 78 => RemoteKey::VolumeUp,
        114 | 12 | 74 => RemoteKey::VolumeDown,
        113 => RemoteKey::Mute,
        _ => return None,
    })
}

/// Spawn one reader thread per input device found at start-up.
pub fn spawn(tx: Sender<RemoteKey>) -> usize {
    let mut count = 0;
    let Ok(dir) = std::fs::read_dir("/dev/input") else { return 0 };
    for entry in dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("event") {
            continue;
        }
        let Ok(mut f) = File::open(entry.path()) else { continue };
        let tx = tx.clone();
        count += 1;
        thread::spawn(move || {
            // struct input_event on 64-bit: timeval(16) + type(2) + code(2) + value(4)
            let mut ev = [0u8; 24];
            while f.read_exact(&mut ev).is_ok() {
                let typ = u16::from_ne_bytes([ev[16], ev[17]]);
                let code = u16::from_ne_bytes([ev[18], ev[19]]);
                let value = i32::from_ne_bytes([ev[20], ev[21], ev[22], ev[23]]);
                // EV_KEY, value 1 = press, 2 = autorepeat
                if typ == 1 && (value == 1 || value == 2) {
                    if let Some(k) = map(code) {
                        if APP_RUNNING.load(Ordering::SeqCst) && k != RemoteKey::Home {
                            continue;
                        }
                        let _ = tx.send(k);
                    }
                }
            }
        });
    }
    count
}
