use crate::color::Color;

/// Theme styling tokens directly inspired by login.ticly.it
pub struct Theme;

impl Theme {
    // Backgrounds
    pub const BG_BASE: Color = Color::rgb(7, 9, 14); // #07090e
    pub const BG_GRADIENT_DARK: Color = Color::rgba(0, 0, 0, 70);

    // Ambient Glowing Orbs
    pub const ORB_AMBER: Color = Color::rgba(245, 158, 11, 45); // #f59e0b
    pub const ORB_VIOLET: Color = Color::rgba(139, 92, 246, 38); // #8b5cf6
    pub const ORB_CYAN: Color = Color::rgba(6, 182, 212, 35); // #06b6d4

    // Glassmorphic Card Surfaces
    pub const CARD_BG: Color = Color::rgba(16, 18, 26, 210); // rgba(16, 18, 26, 0.82)
    pub const CARD_BORDER: Color = Color::rgba(255, 255, 255, 30); // 1px border rgba(255, 255, 255, 0.12)
    pub const CARD_BORDER_HIGHLIGHT: Color = Color::rgba(255, 255, 255, 55); // Top inner glow

    // Focus state (for 10-foot TV Remote navigation)
    pub const FOCUS_BORDER: Color = Color::rgba(245, 158, 11, 230); // Amber glow border
    pub const FOCUS_BG: Color = Color::rgba(30, 32, 48, 235);
    pub const FOCUS_GLOW: Color = Color::rgba(245, 158, 11, 80);

    // Sub-elements / Item containers
    pub const ITEM_BG: Color = Color::rgba(255, 255, 255, 8); // rgba(255, 255, 255, 0.03)
    pub const ITEM_BORDER: Color = Color::rgba(255, 255, 255, 18);

    // Text hierarchy
    pub const TEXT_PRIMARY: Color = Color::white();
    pub const TEXT_SECONDARY: Color = Color::rgba(255, 255, 255, 165); // ~65%
    pub const TEXT_MUTED: Color = Color::rgba(255, 255, 255, 100); // ~40%
    pub const TEXT_AMBER: Color = Color::rgb(251, 191, 36);

    // Accent Colors
    pub const AMBER: Color = Color::rgb(245, 158, 11);
    pub const EMERALD: Color = Color::rgb(16, 185, 129);
    pub const VIOLET: Color = Color::rgb(139, 92, 246);
    pub const CYAN: Color = Color::rgb(6, 182, 212);
    pub const ROSE: Color = Color::rgb(244, 63, 94);

    // Status Badge
    pub const BADGE_BG: Color = Color::rgba(245, 158, 11, 30);
    pub const BADGE_BORDER: Color = Color::rgba(245, 158, 11, 75);
    pub const BADGE_TEXT: Color = Color::rgb(251, 191, 36);
}
