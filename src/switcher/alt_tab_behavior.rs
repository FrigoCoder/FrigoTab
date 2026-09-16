/// Determines what releasing Alt does after the switcher opens.
///
/// Sticky mode keeps the session open for deliberate keyboard or pointer
/// selection. Tap mode follows the classic Alt-Tab gesture and commits the
/// currently selected candidate when Alt is released.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AltTabBehavior {
    #[default]
    Sticky,
    Tap,
}
