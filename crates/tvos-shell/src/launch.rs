//! Starting installed apps. An app owns the screen until it exits; the Home key (remote, phone or keyboard)
//! always closes it, and every other key from the phone / TV remote is typed into it through a virtual keyboard.

use crate::input::APP_RUNNING;
use crate::uinput::UInput;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};
use tvos_apps::Launch;
use tvos_core::input::RemoteKey;

const COG: &str = "/usr/bin/cog";
const DBUS_RUN_SESSION: &str = "/usr/bin/dbus-run-session";
const COG_CONFIG: &str = "/etc/tvos/cog.conf";
const RUNTIME_DIR: &str = "/run/tvos";

#[derive(Default)]
pub struct Runner {
    uinput: Option<UInput>,
}

fn browser_command(url: &str) -> Option<Command> {
    if !Path::new(COG).exists() {
        return None;
    }
    let _ = std::fs::create_dir_all(RUNTIME_DIR);
    let _ = std::fs::set_permissions(RUNTIME_DIR, std::os::unix::fs::PermissionsExt::from_mode(0o700));
    // Cog wants a D-Bus session; dbus-run-session starts a private one for the lifetime of the browser
    let mut cmd = if Path::new(DBUS_RUN_SESSION).exists() {
        let mut c = Command::new(DBUS_RUN_SESSION);
        c.args(["--", COG]);
        c
    } else {
        Command::new(COG)
    };
    cmd.args(["--platform=drm", &format!("--config={COG_CONFIG}"), url])
        .env("XDG_RUNTIME_DIR", RUNTIME_DIR)
        .env("COG_PLATFORM_DRM_MODE_MAX", "1280x720")
        .env("WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS", "1");
    Some(cmd)
}

/// Stop the whole process group (the browser spawns web/network helper processes).
fn kill_tree(child: &mut Child) {
    let pgid = child.id() as i32;
    unsafe { libc::kill(-pgid, libc::SIGTERM) };
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    unsafe { libc::kill(-pgid, libc::SIGKILL) };
    let _ = child.wait();
}

impl Runner {
    /// Run the app and wait for it to exit (or for Home). `Err` carries a message to show on screen.
    pub fn run(&mut self, l: &Launch, keys: &Receiver<RemoteKey>) -> Result<(), String> {
        let (name, mut cmd) = match l {
            Launch::Native { name, exec } => {
                let mut c = Command::new(exec);
                c.current_dir(exec.parent().unwrap_or(Path::new("/")));
                (name, c)
            }
            Launch::Web { name, url } => match browser_command(url) {
                Some(c) => (name, c),
                None => return Err(format!("Per aprire {name} serve il browser, non incluso in questa immagine")),
            },
        };
        cmd.process_group(0); // own group, so Home can stop the app and all its helpers
        if self.uinput.is_none() {
            self.uinput = UInput::create().ok(); // without it phone keys cannot reach the app, but Home still works
        }
        APP_RUNNING.store(true, Ordering::SeqCst);
        let started = Instant::now();
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                APP_RUNNING.store(false, Ordering::SeqCst);
                return Err(format!("Impossibile avviare {name}: {e}"));
            }
        };
        let mut closed_by_user = false;
        let status = loop {
            if let Ok(Some(st)) = child.try_wait() {
                break Some(st);
            }
            match keys.recv_timeout(Duration::from_millis(50)) {
                Ok(RemoteKey::Home) => {
                    closed_by_user = true;
                    kill_tree(&mut child);
                    break None;
                }
                Ok(k) => {
                    if let Some(u) = self.uinput.as_mut() {
                        u.tap(k);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    kill_tree(&mut child);
                    break None;
                }
            }
        };
        APP_RUNNING.store(false, Ordering::SeqCst);
        match status {
            Some(st) if !st.success() => {
                let quick = started.elapsed() < Duration::from_secs(5);
                Err(if quick { format!("{name} non è riuscita ad avviarsi") } else { format!("{name} si è chiusa con un errore") })
            }
            _ if closed_by_user => Ok(()),
            _ => Ok(()),
        }
    }
}
