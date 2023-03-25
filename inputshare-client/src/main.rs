#![windows_subsystem = "windows"]

mod sender;
mod ui;
mod runtime;
mod utils;
mod model;

use std::sync::Arc;
use std::time::Duration;
use bytes::Bytes;
use druid::widget::{Button, Flex, Label, List, SizedBox, TextBox};
use druid::{AppLauncher, EventCtx, ExtEventSink, lens, Widget, WidgetExt, WindowDesc};
use quinn::{ClientConfig, Connection, Endpoint, TransportConfig};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use yawi::{InputHook, VirtualKey};
use tokio::select;
use tokio::time::Instant;
use crate::model::{AppState, Config, ConnectionState, Hotkey, Side};
use crate::runtime::{ExtEventSinkCallback, RuntimeDelegate};
use crate::sender::InputSender;
use crate::ui::popup::Popup;
use crate::ui::{open_key_picker, theme};
use crate::ui::button::WidgetButton;
use crate::ui::icons::Icon;
use crate::ui::theme::Theme;
use crate::utils::{hook, process_hook_event, SkipServerVerification};
use crate::utils::keyset::VirtualKeySet;
use druid_material_icons::normal::hardware::CAST;

pub fn main() {
    tracing_subscriber::registry()
        .with(Targets::new()
            .with_default(LevelFilter::DEBUG)
            .with_target("yawi", LevelFilter::TRACE)
            .with_target("inputshare_client", LevelFilter::TRACE))
        .with(layer()
            .without_time())
        .init();

    #[cfg(not(debug_assertions))]
    error_tools::gui::set_gui_panic_hook();

    let window = WindowDesc::new(make_ui())
        .window_size((450.0, 300.0))
        .title("InputShare Client");

    AppLauncher::with_window(window)
        .delegate(RuntimeDelegate::new())
        .configure_env(|env, _| theme::setup_theme(Theme::Light, env))
        .launch(AppState::default())
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    //let ui = Flex::column()
    //    .with_child(Label::dynamic(|data: &AppState, _| match data.connection_state {
    //        ConnectionState::Connected(Side::Local) => "Local",
    //        ConnectionState::Connected(Side::Remote) => "Remote",
    //        ConnectionState::Disconnected => "Disconnected",
    //        ConnectionState::Connecting => "Connecting"
    //    }.to_string())
    //        .with_text_size(25.0))
    //    .with_spacer(20.0)
    //    .with_child(Button::from_label(Label::dynamic(|data: &AppState, _| match data.connection_state {
    //        ConnectionState::Connected(_) => "Disconnect",
    //        ConnectionState::Disconnected => "Connect",
    //        ConnectionState::Connecting => "Connecting"
    //    }.to_string())
    //        .with_text_size(17.0))
    //        .fix_size(250.0, 65.0)
    //        .on_click(|ctx, _, _| initiate_connection(ctx))
    //        .disabled_if(|data: &AppState, _ | data.connection_state == ConnectionState::Connecting))
    //    .with_default_spacer()
    //    .with_child(Button::dynamic(|data: &AppState, _|data.config.hotkey.trigger.to_string())
    //        .fix_width(250.0)
    //        .on_click(|ctx, _, _ |  open_key_picker(ctx,  |data, key| data.config.hotkey.trigger = key)))
    //    .center();
    let popup = Flex::column()
        .with_child(Label::new("Press any key"))
        .center();
    let popup = SizedBox::new(popup)
        .width(200.0)
        .height(100.0)
        .background(druid::theme::BACKGROUND_DARK)
        .rounded(5.0);
    Popup::new(|data: &AppState, _| data.popup, popup, main_ui())
}

fn main_ui() -> impl Widget<AppState> + 'static {
    Flex::column()
        .with_flex_child(config_ui().lens(AppState::config), 1.0)
        .with_spacer(5.0)
        .with_child(status_ui())
        .padding(5.0)
}

fn config_ui() -> impl Widget<Config> + 'static {
    let host = host_ui()
        .lens(Config::host_address);
    let blacklist = blacklist_ui()
        .lens(Config::blacklist);
    let hotkey = hotkey_ui()
        .lens(Config::hotkey);
    let keys = Flex::row()
        .with_flex_child(hotkey, 1.0)
        .with_spacer(5.0)
        .with_flex_child(blacklist, 1.0);
    Flex::column()
        .with_child(host)
        .with_default_spacer()
        .with_flex_child(keys, 1.0)
}

fn host_ui() -> impl Widget<String> + 'static {
    let host = TextBox::new()
        .expand_width();
    let search = WidgetButton::new(Icon::from(CAST)
        .padding(5.0));
    Flex::row()
        .with_flex_child(host, 1.0)
        .with_spacer(5.0)
        .with_child(search)
}

fn hotkey_ui() -> impl Widget<Hotkey> + 'static {
    let modifiers = keyset_ui()
        .lens(Hotkey::modifiers);
    let trigger = Button::dynamic(|data: &VirtualKey, _| data.to_string())
        .expand_width()
        .lens(Hotkey::trigger);
    Flex::column()
        .with_child(Label::new("Hotkey")
            .expand_width())
        .with_flex_child(modifiers, 1.0)
        .with_child(Label::new("+"))
        .with_child(trigger)
}

fn status_ui() -> impl Widget<AppState> + 'static {
    let status = Label::dynamic(|data: &AppState, _| format!("{:?}", data.connection_state))
        .center()
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
        .expand();
    let button = Button::new("Connect")
        .fix_width(100.0)
        .expand_height();
    Flex::row()
        .with_flex_child(status, 1.0)
        .with_spacer(5.0)
        .with_child(button)
        .fix_height(35.0)
}

fn blacklist_ui() -> impl Widget<VirtualKeySet> + 'static {
    Flex::column()
        .with_child(Label::new("Blacklist")
            .expand_width())
        .with_flex_child(keyset_ui(), 1.0)
}

fn keyset_ui() -> impl Widget<VirtualKeySet> + 'static {
    let list = List::new(key_ui)
        .with_spacing(2.0)
        .padding(2.0);
    let add = Button::new("Add new")
        .fix_height(23.0)
        .padding(2.0)
        .expand_width();
    druid::widget::Scroll::new(
        Flex::column()
            .with_child(list)
            .with_child(add))
        .vertical()
        .expand()
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
}

fn key_ui() -> impl Widget<VirtualKey> + 'static {
    Label::dynamic(|key: &VirtualKey, _| key.to_string())
        .center()
        .padding(2.0)
        .background(druid::theme::BACKGROUND_LIGHT)
        .border(druid::theme::BORDER_DARK, 1.0)
        .rounded(3.0)
        .expand_width()
        .on_click(|_, _, _| println!("remove"))
}

fn initiate_connection(ctx: &mut EventCtx) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| {
        rt.hook = None;
        if std::mem::take(&mut data.connection_state) == ConnectionState::Disconnected{
            data.connection_state = ConnectionState::Connecting;
            rt.runtime.spawn(async move {
                connection(&handle)
                    .await
                    .unwrap_or_else(|err| tracing::warn!("could not establish connection: {}", err));
                handle.add_rt_callback(|rt, data | {
                    rt.hook = None;
                    data.connection_state = ConnectionState::Disconnected;
                });
            });
        }
    })
}


async fn connection(sink: &ExtEventSink) -> anyhow::Result<()> {
    let connection = connect().await?;
    tracing::debug!("Connected to {}", connection.remote_address());
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    sink.add_rt_callback(|rt, data| {
        rt.hook = InputHook::register(hook::create_callback(&data.config, sender))
            .map_err(|err| tracing::warn!("Failed to register hook: {}", err))
            .ok();
    });

    let mut sender = InputSender::new(1.0);
    let mut deadline = None;
    loop {
        let timeout = async move {
            match deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => futures_lite::future::pending().await
            };
        };
        select! {
            datagram = connection.read_datagram() => {
                let datagram: Bytes = datagram?;
                sender.read_packet(&datagram)?;
            },
            event = receiver.recv() => match event {
                Some(event) => process_hook_event(&mut sender, sink, event),
                None => break
            },
            _ = timeout => {
                let msg = sender.write_packet()?;
                debug_assert!(msg.len() <= connection.max_datagram_size().unwrap());
                connection.send_datagram(Bytes::copy_from_slice(msg))?;
                deadline = Some(Instant::now() + Duration::from_millis(10));
            }
        };
        deadline = match sender.in_sync() {
            true => None,
            false => Some(deadline.unwrap_or_else(Instant::now))
        };
    }

    tracing::trace!("Shutting down key handler");

    Ok(())
}


async fn connect() -> anyhow::Result<Connection> {
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();
    let mut transport = TransportConfig::default();
    transport.keep_alive_interval(Some(Duration::from_secs(1)));

    let mut config = ClientConfig::new(Arc::new(crypto));
    config.transport_config(Arc::new(transport));
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(config);

    let connection = endpoint.connect("127.0.0.1:12345".parse()?, "dummy")?.await?;
    Ok(connection)
}

