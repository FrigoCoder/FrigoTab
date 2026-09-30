/// Identifies the physical modifier whose transition produced an event.
/// Keeping left and right keys distinct prevents releasing one key from
/// clearing the state of the other.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyboardModifierKey {
    None,
    Alt,
    LeftAlt,
    RightAlt,
    Shift,
    LeftShift,
    RightShift,
    Control,
    LeftControl,
    RightControl,
}
