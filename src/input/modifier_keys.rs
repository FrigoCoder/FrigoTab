#[derive(Default)]
pub(crate) struct ModifierKeys {
    // There are only three physical variants for each modifier. Keeping
    // these as masks avoids a HashSet allocation in the low-level hook.
    pub(crate) alt: u8,
    pub(crate) shift: u8,
}
