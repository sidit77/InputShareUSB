use druid::widget::{Button, Either, Flex, Label, SizedBox};
use druid::{Env, Insets, Widget, WidgetExt};

use crate::model::{AppState, ConnectionState};
use crate::ui::actions::{initiate_connection, shutdown_server};

pub fn ui() -> impl Widget<AppState> + 'static {
    let status = Label::dynamic(|data: &AppState, _| format!("{:?}", data.connection_state))
        .center()
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
        .expand();
    let connect_button = Button::dynamic(button_label)
        .on_click(|ctx, _, _| initiate_connection(ctx))
        .expand();
    let shutdown_button = Button::new("Shutdown")
        .on_click(|ctx, _, _| shutdown_server(ctx))
        .padding(Insets::new(0.0, 3.0, 0.0, 0.0))
        .expand_width();
    let buttons = Flex::column()
        .with_flex_child(connect_button, 1.0)
        .with_child(Either::new(|data: &AppState, _| data.enable_shutdown, shutdown_button, SizedBox::empty()))
        .fix_width(100.0);
    Flex::row()
        .with_flex_child(status, 1.0)
        .with_spacer(5.0)
        .with_child(buttons)
        .fix_height(70.0)
}

fn button_label(data: &AppState, _: &Env) -> String {
    match data.connection_state {
        ConnectionState::Disconnected => "Connect",
        ConnectionState::Connecting => "Cancel",
        ConnectionState::Connected(_) => "Disconnect"
    }
    .to_string()
}
