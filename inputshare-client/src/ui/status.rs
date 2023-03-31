use druid::widget::{Button, Flex, Label};
use druid::{Widget, WidgetExt};

use crate::model::AppState;
use crate::ui::actions::initiate_connection;

pub fn ui() -> impl Widget<AppState> + 'static {
    let status = Label::dynamic(|data: &AppState, _| format!("{:?}", data.connection_state))
        .center()
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
        .expand();
    let button = Button::new("Connect")
        .fix_width(100.0)
        .on_click(|ctx, _, _| initiate_connection(ctx))
        .expand_height();
    Flex::row()
        .with_flex_child(status, 1.0)
        .with_spacer(5.0)
        .with_child(button)
        .fix_height(50.0)
}
