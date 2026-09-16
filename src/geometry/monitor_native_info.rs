use super::screen_rectangle::ScreenRectangle;

#[derive(Clone, Debug)]
pub(super) struct MonitorNativeInfo {
    pub(super) id: String,
    pub(super) bounds: ScreenRectangle,
    pub(super) working_area: ScreenRectangle,
}
