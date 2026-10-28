use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;

pub trait App {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn subtitle(&self) -> &'static str;
    fn icon_symbol(&self) -> &'static str;

    fn update(&mut self);
    fn handle_input(&mut self, key: RemoteKey) -> bool; // returns true if handled
    fn render(&self, surface: &mut Surface);
}
