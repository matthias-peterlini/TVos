//! HDMI-CEC OpCodes and User Control Operand definitions (CEC 1.4 & 2.0).

pub const CEC_OPCODE_USER_CONTROL_PRESSED: u8 = 0x44;
pub const CEC_OPCODE_USER_CONTROL_RELEASED: u8 = 0x45;
pub const CEC_OPCODE_ACTIVE_SOURCE: u8 = 0x82;
pub const CEC_OPCODE_STANDBY: u8 = 0x36;

// CEC UI Key operands
pub const CEC_KEY_SELECT: u8 = 0x00;
pub const CEC_KEY_UP: u8 = 0x01;
pub const CEC_KEY_DOWN: u8 = 0x02;
pub const CEC_KEY_LEFT: u8 = 0x03;
pub const CEC_KEY_RIGHT: u8 = 0x04;
pub const CEC_KEY_ROOT_MENU: u8 = 0x09;
pub const CEC_KEY_SETUP_MENU: u8 = 0x0A;
pub const CEC_KEY_CONTENTS_MENU: u8 = 0x0B;
pub const CEC_KEY_BACK_EXIT: u8 = 0x0D;

pub const CEC_KEY_VOLUME_UP: u8 = 0x41;
pub const CEC_KEY_VOLUME_DOWN: u8 = 0x42;
pub const CEC_KEY_MUTE: u8 = 0x43;
pub const CEC_KEY_PLAY: u8 = 0x44;
pub const CEC_KEY_STOP: u8 = 0x45;
pub const CEC_KEY_PAUSE: u8 = 0x46;
pub const CEC_KEY_REWIND: u8 = 0x48;
pub const CEC_KEY_FAST_FORWARD: u8 = 0x49;

pub const CEC_KEY_BLUE: u8 = 0x71;
pub const CEC_KEY_RED: u8 = 0x72;
pub const CEC_KEY_GREEN: u8 = 0x73;
pub const CEC_KEY_YELLOW: u8 = 0x74;
