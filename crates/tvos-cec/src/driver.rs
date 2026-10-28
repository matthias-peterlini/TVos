//! HDMI-CEC through the Linux CEC framework (`/dev/cec0`).
//!
//! The driver registers TVos as a "Playback" device, announces itself as the active source so the TV
//! forwards its remote's keys to us, and turns "User Control Pressed" messages into `RemoteKey`s.
//! If no adapter exists (or the TV is off) the thread keeps retrying; the phone remote keeps working.

use std::sync::mpsc::Sender;
use std::thread;
use tvos_core::input::RemoteKey;

pub struct CecDriver {
    event_tx: Sender<RemoteKey>,
}

impl CecDriver {
    pub fn new(event_tx: Sender<RemoteKey>) -> Self {
        Self { event_tx }
    }

    /// Spawns the CEC monitor thread
    pub fn start(&self) {
        let _tx = self.event_tx.clone();
        thread::spawn(move || {
            #[cfg(target_os = "linux")]
            linux::run(&_tx);
        });
    }
}

/// What to do with a received CEC message (header byte included in `msg`).
#[derive(Debug, PartialEq, Eq)]
pub enum CecAction {
    Key(RemoteKey),
    /// The TV asked who the active source is (Set Stream Path / Request Active Source): answer with Active Source
    AnnounceActiveSource,
    None,
}

/// Decode one received message: `msg[0]` is the header, `msg[1]` the opcode, the rest are operands.
pub fn decode(msg: &[u8]) -> CecAction {
    if msg.len() < 2 {
        return CecAction::None;
    }
    match msg[1] {
        crate::cec_codes::CEC_OPCODE_USER_CONTROL_PRESSED => {
            crate::parser::parse_cec_frame(msg[1], msg.get(2).copied()).map_or(CecAction::None, CecAction::Key)
        }
        0x85 | 0x86 => CecAction::AnnounceActiveSource,
        _ => CecAction::None,
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::os::unix::io::AsRawFd;
    use std::time::Duration;

    // _IOC(dir, 'a', nr, size): see include/uapi/linux/cec.h
    const CEC_ADAP_G_PHYS_ADDR: u32 = 0x8002_6101;
    const CEC_ADAP_G_LOG_ADDRS: u32 = 0x805C_6103;
    const CEC_ADAP_S_LOG_ADDRS: u32 = 0xC05C_6104;
    const CEC_TRANSMIT: u32 = 0xC038_6105;
    const CEC_RECEIVE: u32 = 0xC038_6106;
    const CEC_S_MODE: u32 = 0x4004_6109;

    const CEC_MODE_INITIATOR: u32 = 0x1;
    const CEC_MODE_FOLLOWER: u32 = 0x10;
    const STATUS_FILE: &str = "/run/tvos-cec";

    // struct cec_msg is 56 bytes: len@16, timeout@20, msg[16]@32
    struct Msg([u8; 56]);

    impl Msg {
        fn new() -> Self {
            Msg([0; 56])
        }
        fn with(bytes: &[u8]) -> Self {
            let mut m = Self::new();
            m.0[16..20].copy_from_slice(&(bytes.len() as u32).to_ne_bytes());
            m.0[20..24].copy_from_slice(&1000u32.to_ne_bytes()); // ms
            m.0[32..32 + bytes.len()].copy_from_slice(bytes);
            m
        }
        fn payload(&self) -> &[u8] {
            let len = u32::from_ne_bytes(self.0[16..20].try_into().unwrap()).min(16) as usize;
            &self.0[32..32 + len]
        }
    }

    fn set_status(ok: bool) {
        let _ = fs::write(STATUS_FILE, if ok { "ok" } else { "down" });
    }

    fn ioctl<T>(fd: i32, req: u32, arg: *mut T) -> bool {
        unsafe { libc::ioctl(fd, req as _, arg) >= 0 }
    }

    /// Claim a Playback logical address. Returns it, or None while the TV/HDMI link is not ready.
    fn register(fd: i32) -> Option<u8> {
        let mut la = [0u8; 92]; // struct cec_log_addrs
        la[6] = 5; // cec_version: 1.4
        la[7] = 1; // num_log_addrs
        la[8..12].copy_from_slice(&0x00FF_FFFFu32.to_ne_bytes()); // vendor id: none
        la[16..20].copy_from_slice(b"TVos"); // osd_name
        la[31] = 4; // primary_device_type: playback
        la[35] = 3; // log_addr_type: playback
        la[39] = 0x10; // all_device_types: playback (CEC_OP_ALL_DEVTYPE_PLAYBACK)
        if !ioctl(fd, CEC_ADAP_S_LOG_ADDRS, la.as_mut_ptr()) || !ioctl(fd, CEC_ADAP_G_LOG_ADDRS, la.as_mut_ptr()) {
            return None;
        }
        let addr = la[0];
        (addr < 15).then_some(addr)
    }

    fn transmit(fd: i32, bytes: &[u8]) {
        let mut m = Msg::with(bytes);
        let _ = ioctl(fd, CEC_TRANSMIT, m.0.as_mut_ptr());
    }

    fn announce(fd: i32, la: u8) {
        let mut pa = [0u8; 2];
        if !ioctl(fd, CEC_ADAP_G_PHYS_ADDR, pa.as_mut_ptr()) {
            return;
        }
        let pa = u16::from_ne_bytes(pa);
        // Image View On (to the TV), then Active Source (broadcast, physical address big-endian)
        transmit(fd, &[la << 4, 0x04]);
        transmit(fd, &[(la << 4) | 0xF, 0x82, (pa >> 8) as u8, pa as u8]);
    }

    pub fn run(tx: &Sender<RemoteKey>) {
        loop {
            set_status(false);
            let Ok(file) = OpenOptions::new().read(true).write(true).open("/dev/cec0") else {
                thread::sleep(Duration::from_secs(10));
                continue;
            };
            let fd = file.as_raw_fd();
            let mut mode = CEC_MODE_INITIATOR | CEC_MODE_FOLLOWER;
            if !ioctl(fd, CEC_S_MODE, &mut mode) {
                thread::sleep(Duration::from_secs(10));
                continue;
            }
            let la = loop {
                if let Some(a) = register(fd) {
                    break a;
                }
                thread::sleep(Duration::from_secs(3)); // TV off or HDMI not connected yet
            };
            announce(fd, la);
            set_status(true);

            // Receive loop: 1 s timeout so a vanished adapter is noticed
            loop {
                let mut m = Msg::with(&[]);
                if !ioctl(fd, CEC_RECEIVE, m.0.as_mut_ptr()) {
                    match std::io::Error::last_os_error().raw_os_error() {
                        Some(libc::ETIMEDOUT) | Some(libc::EINTR) => continue,
                        _ => break, // adapter gone: re-register
                    }
                }
                match decode(m.payload()) {
                    CecAction::Key(k) => {
                        let _ = tx.send(k);
                    }
                    CecAction::AnnounceActiveSource => announce(fd, la),
                    CecAction::None => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_control_pressed_maps_to_keys() {
        // header 0x40 = TV (0) -> playback (4); opcode 0x44; operand
        assert_eq!(decode(&[0x04, 0x44, 0x01]), CecAction::Key(RemoteKey::Up));
        assert_eq!(decode(&[0x04, 0x44, 0x00]), CecAction::Key(RemoteKey::Select));
        assert_eq!(decode(&[0x04, 0x44, 0x0D]), CecAction::Key(RemoteKey::Back));
        assert_eq!(decode(&[0x04, 0x44, 0x44]), CecAction::Key(RemoteKey::PlayPause));
        assert_eq!(decode(&[0x04, 0x44, 0x23]), CecAction::Key(RemoteKey::Number(3)));
    }

    #[test]
    fn unknown_or_short_messages_do_nothing() {
        assert_eq!(decode(&[]), CecAction::None);
        assert_eq!(decode(&[0x04]), CecAction::None);
        assert_eq!(decode(&[0x04, 0x44]), CecAction::None); // pressed without operand
        assert_eq!(decode(&[0x04, 0x44, 0x7F]), CecAction::None); // unmapped key
        assert_eq!(decode(&[0x04, 0x45, 0x01]), CecAction::None); // released
    }

    #[test]
    fn tv_asking_for_active_source_is_answered() {
        assert_eq!(decode(&[0x0F, 0x85]), CecAction::AnnounceActiveSource);
        assert_eq!(decode(&[0x0F, 0x86, 0x10, 0x00]), CecAction::AnnounceActiveSource);
    }
}
