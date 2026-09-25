/// Controls the close button rendered on each application thumbnail.
///
/// `AlwaysVisible` preserves FrigoTab's current close-button appearance.
/// `HoverOnly` follows the Windows Alt-Tab and Task View (Win-Tab) behavior:
/// only the hovered thumbnail exposes a white X without a background.  The
/// hidden mode keeps the close control out of the thumbnail altogether.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CloseButtonMode {
    #[default]
    AlwaysVisible,
    HoverOnly,
    Hidden,
}
