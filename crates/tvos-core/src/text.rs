//! TTF text rendering (Inter) with a glyph cache. Anti-aliased, proportional.

use fontdue::{Font, FontSettings, Metrics};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Weight {
    Regular,
    SemiBold,
    Bold,
    /// Google Sans Flex, widest and slightly rounded: clock and greeting
    Display,
}

static FONTS: OnceLock<[Font; 4]> = OnceLock::new();

type Glyph = Arc<(Metrics, Vec<u8>)>;
type Cache = Mutex<HashMap<(char, u32, Weight), Glyph>>;
static CACHE: OnceLock<Cache> = OnceLock::new();

fn fonts() -> &'static [Font; 4] {
    FONTS.get_or_init(|| {
        let load = |b: &[u8]| Font::from_bytes(b, FontSettings::default()).expect("font");
        [
            load(include_bytes!("../assets/Inter-Regular.ttf")),
            load(include_bytes!("../assets/Inter-SemiBold.ttf")),
            load(include_bytes!("../assets/Inter-Bold.ttf")),
            load(include_bytes!("../assets/GoogleSansFlex-Clock.ttf")),
        ]
    })
}

fn font(w: Weight) -> &'static Font {
    &fonts()[w as usize]
}

pub fn glyph(c: char, size: f32, w: Weight) -> Glyph {
    let key = (c, (size * 4.0) as u32, w);
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().unwrap();
    map.entry(key)
        .or_insert_with(|| Arc::new(font(w).rasterize(c, size)))
        .clone()
}

pub fn ascent(size: f32, w: Weight) -> f32 {
    font(w).horizontal_line_metrics(size).map(|m| m.ascent).unwrap_or(size * 0.9)
}

pub fn text_width(text: &str, size: f32, w: Weight) -> i32 {
    let f = font(w);
    text.chars()
        .filter(|c| *c != '\n')
        .map(|c| f.metrics(c, size).advance_width)
        .sum::<f32>()
        .ceil() as i32
}
