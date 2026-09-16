use std::rc::Rc;

use super::owned_icon::OwnedIcon;

pub(super) type ChangedHandler = Rc<dyn Fn() + 'static>;

pub(super) struct IconState {
    pub(super) disposed: bool,
    pub(super) icon: Option<OwnedIcon>,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) changed: Option<ChangedHandler>,
}
