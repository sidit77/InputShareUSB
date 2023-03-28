#![windows_subsystem = "windows"]

mod sender;
mod ui;
mod runtime;
mod utils;
mod model;

use std::mem::Discriminant;
use std::sync::Arc;
use std::time::Duration;
use bytes::Bytes;
use druid::widget::{BackgroundBrush, Button, CrossAxisAlignment, Flex, Label, List, Maybe, Scroll, TextBox, ViewSwitcher};
use druid::{AppLauncher, Color, EventCtx, ExtEventSink, Widget, WidgetExt, WindowDesc};
use druid::im::Vector;
use quinn::{ClientConfig, Connection, Endpoint, TransportConfig};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use yawi::{InputHook, VirtualKey};
use tokio::select;
use tokio::time::Instant;
use crate::model::{AppState, Config, ConnectionState, Hotkey, PopupType, SearchResult};
use crate::runtime::{ExtEventSinkCallback, RuntimeDelegate};
use crate::sender::InputSender;
use crate::ui::{open_key_picker, theme};
use crate::ui::button::WidgetButton;
use crate::ui::icons::Icon;
use crate::ui::theme::Theme;
use crate::utils::{hook, process_hook_event, SkipServerVerification};
use crate::utils::keyset::VirtualKeySet;
use druid_material_icons::normal::hardware::CAST;
use druid_material_icons::normal::content::ADD;
use searchlight::discovery::{Discovery, DiscoveryEvent};
use searchlight::net::IpVersion;
use crate::ui::list::WrappingList;

pub fn main() {
    tracing_subscriber::registry()
        .with(Targets::new()
            .with_default(LevelFilter::DEBUG)
            .with_target("yawi", LevelFilter::TRACE)
            .with_target("inputshare_client", LevelFilter::TRACE)
            .with_target("inputshare_client::ui::list", LevelFilter::DEBUG))
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
    //Popup::new(|data: &AppState, _| data.popup != PopupType::None, popup_ui().lens(AppState::popup), main_ui())
    let popup = Maybe::or_empty(||popup_ui())
        .lens(AppState::popup);
    druid::widget::ZStack::new(main_ui())
        .with_centered_child(popup)
}

fn popup_ui() -> impl Widget<PopupType> + 'static {
    ViewSwitcher::<PopupType, Discriminant<PopupType>>::new(|t, _| std::mem::discriminant(t), |_, s, _| match s {
        PopupType::Searching(_) => {
            Flex::column()
                .with_child(List::new(||Label::dynamic(|res: &SearchResult, _| res.addrs.to_string())))
                .with_child(Button::new("Back")
                    .on_click(|ctx, _, _ | ctx.add_rt_callback(|rt, data| {
                        if let Some(task) = &rt.mdns {
                            task.abort();
                        }
                        rt.mdns = None;
                        data.popup = None
                    })))
                .center()
                .fix_size(200.0, 100.0)
                .background(druid::theme::BACKGROUND_DARK)
                .rounded(5.0)
                .lens(PopupType::search())
                .boxed()
        }
        PopupType::PressKey => key_popup_ui().boxed()
    })
        .center()
        .background(BackgroundBrush::Color(Color::rgba8(0, 0, 0, 128)))
        .expand()
}

fn key_popup_ui() -> impl Widget<PopupType> + 'static {
    Label::new("Press any key")
        .center()
        .fix_size(200.0, 100.0)
        .background(druid::theme::BACKGROUND_DARK)
        .rounded(5.0)
}

fn main_ui() -> impl Widget<AppState> + 'static {
    let config = config_ui()
        .lens(AppState::config)
        .disabled_if(|data, _|data.connection_state != ConnectionState::Disconnected);
    Flex::column()
        .with_flex_child(config, 1.0)
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
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(host)
        .with_default_spacer()
        .with_child(Label::new("Hotkey"))
        .with_child(hotkey)
        .with_default_spacer()
        .with_child(Label::new("Blacklist"))
        .with_flex_child(blacklist, 1.0)
}

fn host_ui() -> impl Widget<String> + 'static {
    let host = TextBox::new()
        .expand_width();
    let search = WidgetButton::new(Icon::from(CAST)
        .padding(5.0))
        .on_click(|ctx,_,_|start_search(ctx));
    Flex::row()
        .with_flex_child(host, 1.0)
        .with_spacer(5.0)
        .with_child(search)
}

fn hotkey_ui() -> impl Widget<Hotkey> + 'static {
    let add = add_button(|data, key| {
        let hotkey = &mut data.config.hotkey;
        match key != hotkey.trigger {
            true => hotkey.modifiers.insert(key),
            false => tracing::warn!("The trigger can not be a modifier")
        }
    });
    let list = WrappingList::new(key_ui)
        .with_end(add)
        .horizontal()
        .with_spacing(2.0)
        .padding(2.0);
    let modifiers = Scroll::new(list)
        .horizontal()
        .lens(Hotkey::modifiers);
    let trigger = Button::dynamic(|data: &VirtualKey, _| data.to_string())
        .on_click(|ctx, _, _| open_key_picker(ctx, |data, key| {
            let hotkey = &mut data.config.hotkey;
            hotkey.modifiers.remove(key);
            hotkey.trigger = key;
        }))
        .lens(Hotkey::trigger);
    Flex::row()
        .with_child(modifiers)
        .with_child(Icon::from(ADD))
        .with_child(trigger)
        .with_spacer(2.0)
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
}

fn blacklist_ui() -> impl Widget<VirtualKeySet> + 'static {
    let add = add_button(|data, key| data.config.blacklist.insert(key));
    let list = WrappingList::new(key_ui)
        .with_end(add)
        .horizontal()
        .with_spacing(2.0)
        .padding(2.0);
    Scroll::new(list)
        .vertical()
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
}

fn add_button(setter: fn(&mut AppState, VirtualKey)) -> impl Widget<()> + 'static {
    Button::new("+")
        .env_scope(|env, _| {
            env.set(druid::theme::BUTTON_DARK, Color::TRANSPARENT);
            env.set(druid::theme::BUTTON_LIGHT, Color::TRANSPARENT);
            env.set(druid::theme::DISABLED_BUTTON_DARK, Color::TRANSPARENT);
            env.set(druid::theme::DISABLED_BUTTON_LIGHT, Color::TRANSPARENT);
        })
        .on_click(move |ctx, _, _| open_key_picker(ctx, setter))
}

fn key_ui() -> impl Widget<(VirtualKeySet, VirtualKey)> + 'static {
    Button::<(VirtualKeySet, VirtualKey)>::dynamic(|(_, key): &(_, VirtualKey), _| key.to_string())
        .on_click(|_, (set, key), _| set.remove(*key))
}

fn status_ui() -> impl Widget<AppState> + 'static {
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

fn start_search(ctx: &mut EventCtx) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| {
        let task = rt.runtime.spawn(async move {
            if let Err(err) = search(handle.clone()).await {
                tracing::error!("mdns search failed: {}", err);
                handle.add_rt_callback(|rt, data| {
                   rt.mdns = None;
                   data.popup = None;
                });
            }
        });
        rt.mdns = Some(task);
        data.popup = Some(PopupType::Searching(Vector::new()))
    })
}

async fn search(ctx: ExtEventSink) -> anyhow::Result<()> {
    Discovery::builder()
        .service("_http._tcp.local.")?
        .build(IpVersion::Both)?
        .run_async(move |event| {
            ctx.add_idle_callback(move |data: &mut AppState| {
                if let Some(PopupType::Searching(results)) = &mut data.popup {
                    match event {
                        DiscoveryEvent::ResponderFound(resp) => {
                            results.push_back(SearchResult {
                                addrs: resp.addr,
                            });
                        }
                        DiscoveryEvent::ResponderLost(_) => {}
                        DiscoveryEvent::ResponseUpdate { .. } => {}
                    }
                }
            });
        })
        .await?;
    Ok(())
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
                None => std::future::pending().await
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

