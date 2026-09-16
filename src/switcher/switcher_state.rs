/// User-visible lifecycle states of a switcher session.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SwitcherState {
    #[default]
    Idle,
    Visible,
}
