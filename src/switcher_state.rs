/// User-visible lifecycle states of a switcher session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitcherState {
    Idle,
    Visible,
}
