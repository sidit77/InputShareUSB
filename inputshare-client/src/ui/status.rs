use druid::theme::TEXT_COLOR;
use druid::widget::{Button, Either, Flex, Label, Maybe, SizedBox};
use druid::{Color, Env, Insets, Lens, LensExt, Widget, WidgetExt};

use crate::model::{AppState, ConnectionState, Side};
use crate::ui::actions::{initiate_connection, shutdown_server};

#[rustfmt::skip]
pub fn ui() -> impl Widget<AppState> + 'static {
    let status = Flex::column()
        .with_child(Label::dynamic(connection_status)
            .with_text_size(15.0))
        .with_child(Maybe::or_empty(side_label).lens(side_lens()))
        .center()
        .expand()
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0);
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
    let info = Label::new("info")
        .center()
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
        .expand_height()
        .fix_width(80.0)
        .padding(Insets::new(0.0, 0.0, 3.0, 0.0));
    Flex::row()
        .with_child(Either::new(|data: &AppState, _| data.config.show_network_info, info, SizedBox::empty()))
        .with_flex_child(status, 1.0)
        .with_spacer(3.0)
        .with_child(buttons)
        .fix_height(70.0)
}

#[rustfmt::skip]
fn side_label() -> impl Widget<Side> + 'static {
    Label::dynamic(|side: &Side, _| format!("{:?}", side))
        .with_text_size(25.0)
        .env_scope(|env, data: &Side| env.set(TEXT_COLOR, match data {
            Side::Local => Color::BLUE,
            Side::Remote => Color::RED
        }))
}

fn button_label(data: &AppState, _: &Env) -> String {
    match data.connection_state {
        ConnectionState::Disconnected => "Connect",
        ConnectionState::Connecting => "Cancel",
        ConnectionState::Connected(_) => "Disconnect"
    }
    .to_string()
}

fn connection_status(data: &AppState, _: &Env) -> String {
    match data.connection_state {
        ConnectionState::Connected(_) => "Connected",
        ConnectionState::Connecting => "Connecting",
        ConnectionState::Disconnected => "Disconnected"
    }
    .to_string()
}

fn side_lens() -> impl Lens<AppState, Option<Side>> {
    druid::lens::Identity.map(
        |data: &AppState| match data.connection_state {
            ConnectionState::Connected(s) => Some(s),
            _ => None
        },
        |_, _| {}
    )
}
