#![windows_subsystem = "windows"]

mod theme;
mod hook;
mod conversions;
mod popup;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use druid::widget::{Button, Flex, Label, SizedBox};
use druid::{AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, ExtEventSink, Handled, Selector, Target, Widget, WidgetExt, WindowDesc};
use druid::im::HashSet;
use error_tools::log::LogResultExt;
use futures_lite::FutureExt;
use quinn::{ClientConfig, Connection, ConnectionError, Endpoint, TransportConfig};
use tokio::runtime::{Builder, Runtime};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use yawi::{InputEvent, InputHook, ScrollDirection, VirtualKey};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::UnboundedReceiver;
use crate::conversions::{f32_to_i8, vk_to_mb, wsc_to_cdc, wsc_to_hkc};
use crate::hook::HookEvent;
use crate::popup::{Popup};
use crate::theme::Theme;

#[derive(Debug, Clone, Serialize, Deserialize, Data)]
pub struct Hotkey {
    pub modifiers: HashSet<VirtualKey>,
    pub trigger: VirtualKey
}

impl Hotkey {
    pub fn new<T: IntoIterator<Item = VirtualKey>>(modifiers: T, trigger: VirtualKey) -> Self {
        Self { modifiers: HashSet::from_iter(modifiers), trigger}
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Data)]
pub struct Config {
    pub hotkey: Hotkey,
    pub blacklist: HashSet<VirtualKey>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: Hotkey::new(None, VirtualKey::Apps),
            blacklist: HashSet::from([
                VirtualKey::VolumeDown,
                VirtualKey::VolumeUp,
                VirtualKey::VolumeMute,
                VirtualKey::MediaStop,
                VirtualKey::MediaPrevTrack,
                VirtualKey::MediaPlayPause,
                VirtualKey::MediaNextTrack
            ].as_slice()),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Data)]
enum Side {
    Local, Remote
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Data)]
enum ConnectionState {
    Connected(Side),
    #[default]
    Disconnected
}

#[derive(Default, Debug, Clone, Data)]
struct AppState {
    config: Config,
    connection_state: ConnectionState,
    popup: bool
}

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
        .window_size((300.0, 230.0))
        .title("InputShare Client");

    AppLauncher::with_window(window)
        .delegate(RuntimeDelegate::new())
        .configure_env(|env, _| theme::setup_theme(Theme::Light, env))
        .launch(AppState::default())
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    let ui = Flex::column()
        .with_child(Label::dynamic(|data: &AppState, _| match data.connection_state {
            ConnectionState::Connected(Side::Local) => "Local",
            ConnectionState::Connected(Side::Remote) => "Remote",
            ConnectionState::Disconnected => "Disconnected"
        }.to_string())
            .with_text_size(25.0))
        .with_spacer(20.0)
        .with_child(Button::from_label(Label::dynamic(|data: &AppState, _| match data.connection_state {
            ConnectionState::Connected(_) => "Disconnect",
            ConnectionState::Disconnected => "Connect"
        }.to_string())
            .with_text_size(17.0))
            .fix_size(250.0, 65.0)
            .on_click(|ctx, _, _| ctx.submit_command(MSG.with(()))))
        .with_default_spacer()
        .with_child(Button::new("popup")
            .fix_width(250.0)
            .on_click(|_, data: &mut AppState, _ | data.popup = true))
        .center();
    let popup = Flex::column()
        .with_child(Label::new("Hello"))
        .with_child(Button::new("Back").on_click(|_, data: &mut AppState, _| data.popup = false))
        .center();
    let popup = SizedBox::new(popup)
        .width(200.0)
        .height(100.0)
        .background(druid::theme::BACKGROUND_DARK)
        .rounded(5.0);
    Popup::new(|data: &AppState, _| data.popup, popup, ui)
}

pub const MSG: Selector<()> = Selector::new("inputshare.msg");
pub const RESET: Selector<()> = Selector::new("inputshare.reset");

struct RuntimeDelegate {
    hook: Option<InputHook>,
    runtime: Runtime
}

impl RuntimeDelegate {

    fn new() -> Self {
        Self {
            hook: None,
            runtime: Builder::new_multi_thread()
                .enable_all()
                .worker_threads(1)
                .build()
                .expect("Could not start async runtime"),
        }
    }

}

impl AppDelegate<AppState> for RuntimeDelegate {
    fn command(&mut self, ctx: &mut DelegateCtx, _target: Target, cmd: &Command, data: &mut AppState, _env: &Env) -> Handled {
        match cmd {
            cmd if cmd.is(MSG) => {
                data.connection_state = ConnectionState::Disconnected;
                self.hook = match self.hook.take() {
                    None => {
                        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
                        let hook = InputHook::register(hook::create_callback(&data.config, sender))
                            .log_ok("Failed to register hook");
                        if hook.is_some() {
                            let handle = ctx.get_external_handle();
                            self.runtime.spawn(async move {
                                connection(receiver, &handle)
                                    .await
                                    .log_ok("Connection error");
                                handle.submit_command(RESET, Box::new(()),Target::Auto)
                                    .log_ok("Can not submit reset command");
                            });
                        }
                        hook
                    },
                    Some(_) => None
                };
                Handled::Yes
            },
            cmd if cmd.is(RESET) => {
                self.hook = None;
                data.connection_state = ConnectionState::Disconnected;
                Handled::Yes
            }
            _ => Handled::No
        }
    }
}

async fn connection(mut receiver: UnboundedReceiver<HookEvent>, sink: &ExtEventSink) -> anyhow::Result<()> {
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
    tracing::debug!("Connected to {}", connection.remote_address());
    loop {
        let connection_error = async {
            Err(connection.closed().await)
        };
        let next_event = async {
            receiver
                .recv()
                .await
                .ok_or(ConnectionError::LocallyClosed)
        };
        match connection_error.or(next_event).await {
            Ok(event) => match event {
                HookEvent::Captured(captured) => sink.add_idle_callback(move |data: &mut AppState| {
                    data.connection_state = ConnectionState::Connected(match captured {
                        true => Side::Remote,
                        false => Side::Local
                    });
                }),
                HookEvent::Input(event) => match event {
                    InputEvent::MouseMoveEvent(_x, _y) => {

                    }
                    event => {
                        let mut send = connection
                            .open_uni()
                            .await?;

                        send.write_all(format!("{:?}", event).as_bytes()).await?;
                        send.finish().await?;
                    }
                }
            }
            Err(closed) => {
                tracing::debug!("{:?}", closed);
                break;
            }
        }
    }
    tracing::trace!("Shutting down key handler");

    Ok(())
}

async fn forward_keys(mut receiver: UnboundedReceiver<HookEvent>, connection: Connection, sink: ExtEventSink) -> anyhow::Result<()> {

    Ok(())
}

async fn key_handler(mut receiver: UnboundedReceiver<HookEvent>, sink: ExtEventSink) {
    while let Some(event) = receiver.recv().await {
        match event {
            HookEvent::Captured(captured) => sink.add_idle_callback(move |data: &mut AppState| {
                data.connection_state = ConnectionState::Connected(match captured {
                    true => Side::Remote,
                    false => Side::Local
                });
            }),
            HookEvent::Input(event) => match event {
                InputEvent::MouseMoveEvent(_x, _y) => {

                }
                InputEvent::KeyboardKeyEvent(vk, sc, ks) => match wsc_to_hkc(sc) {
                    Some(kc) => tracing::info!("Key {:?} {:?}", kc, ks),
                    None => match wsc_to_cdc(sc){
                        Some(cdc) => tracing::info!("Consumer {:?} {:?}", cdc, ks),
                        None => if! matches!(sc, 0x21d) {
                            tracing::warn!("Unknown key: {} ({:x})", vk, sc)
                        }
                    }
                }
                InputEvent::MouseButtonEvent(mb, ks) => match vk_to_mb(mb) {
                    Some(button) => tracing::info!("Mouse {:?} {:?}", button, ks),
                    None => tracing::warn!("Unknown mouse button: {}", mb)
                }
                InputEvent::MouseWheelEvent(sd) => match sd {
                    ScrollDirection::Horizontal(amount) => tracing::info!("HScroll {:?}", f32_to_i8(amount)),
                    ScrollDirection::Vertical(amount) => tracing::info!("VScroll {:?}", f32_to_i8(amount))
                }
            }
        }
    }
    tracing::trace!("Shutting down key handler");
}


struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}