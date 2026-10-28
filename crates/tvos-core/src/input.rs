#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteKey {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Home,
    PlayPause,
    VolumeUp,
    VolumeDown,
    Mute,
    ColorRed,
    ColorGreen,
    ColorYellow,
    ColorBlue,
    Number(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub key: RemoteKey,
    pub state: KeyState,
}

impl InputEvent {
    pub fn pressed(key: RemoteKey) -> Self {
        Self {
            key,
            state: KeyState::Pressed,
        }
    }

    pub fn released(key: RemoteKey) -> Self {
        Self {
            key,
            state: KeyState::Released,
        }
    }
}
