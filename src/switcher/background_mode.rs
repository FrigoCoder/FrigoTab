/// Controls what `SessionWindow` paints behind the application previews.
///
/// The full Explorer desktop (including icons) is the historical behavior and
/// remains the default. `ImageOnly` asks User32 to paint only the desktop
/// pattern or wallpaper, while `Black` deliberately avoids touching the shell
/// or any screen DC.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BackgroundMode {
    #[default]
    FullDesktop,
    ImageOnly,
    Black,
}
