use crate::cec_codes::*;
use tvos_core::input::RemoteKey;

/// Parses a raw HDMI-CEC packet into a TVos RemoteKey
pub fn parse_cec_frame(opcode: u8, operand: Option<u8>) -> Option<RemoteKey> {
    if opcode != CEC_OPCODE_USER_CONTROL_PRESSED {
        return None;
    }

    let code = operand?;
    match code {
        CEC_KEY_SELECT => Some(RemoteKey::Select),
        CEC_KEY_UP => Some(RemoteKey::Up),
        CEC_KEY_DOWN => Some(RemoteKey::Down),
        CEC_KEY_LEFT => Some(RemoteKey::Left),
        CEC_KEY_RIGHT => Some(RemoteKey::Right),
        CEC_KEY_BACK_EXIT => Some(RemoteKey::Back),
        CEC_KEY_ROOT_MENU | CEC_KEY_SETUP_MENU => Some(RemoteKey::Home),
        CEC_KEY_PLAY | CEC_KEY_PAUSE => Some(RemoteKey::PlayPause),
        CEC_KEY_VOLUME_UP => Some(RemoteKey::VolumeUp),
        CEC_KEY_VOLUME_DOWN => Some(RemoteKey::VolumeDown),
        CEC_KEY_MUTE => Some(RemoteKey::Mute),
        CEC_KEY_RED => Some(RemoteKey::ColorRed),
        CEC_KEY_GREEN => Some(RemoteKey::ColorGreen),
        CEC_KEY_YELLOW => Some(RemoteKey::ColorYellow),
        CEC_KEY_BLUE => Some(RemoteKey::ColorBlue),
        0x20..=0x29 => Some(RemoteKey::Number(code - 0x20)),
        _ => None,
    }
}
