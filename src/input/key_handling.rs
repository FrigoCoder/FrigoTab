/// The decision returned to the keyboard-hook adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyHandling {
    PassThrough,
    Consume,
}
