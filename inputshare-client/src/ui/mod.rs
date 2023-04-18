use druid::widget::{Flex, Maybe};
use druid::{Widget, WidgetExt};

use crate::model::{AppState, ConnectionState};

mod actions;
mod config;
mod popup;
mod status;
pub mod widget;

#[rustfmt::skip]
pub fn ui() -> impl Widget<AppState> {
    let popup = Maybe::or_empty(popup::ui)
        .lens(AppState::popup);
    let config = config::ui()
        .lens(AppState::config)
        .disabled_if(|data, _| data.connection_state != ConnectionState::Disconnected);
    let main = Flex::column()
        .with_flex_child(config, 1.0)
        .with_spacer(5.0)
        .with_child(status::ui())
        .padding(5.0);
    druid::widget::ZStack::new(main)
        .with_centered_child(popup)
}
