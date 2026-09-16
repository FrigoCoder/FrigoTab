#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrayMenuItem {
    pub label: String,
    pub checked: bool,
    pub children: Vec<TrayMenuItem>,
}
