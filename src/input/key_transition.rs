/// Whether a keyboard event represents a press or a release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyTransition {
    Down,
    Up,
}
