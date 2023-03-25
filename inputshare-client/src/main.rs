#![windows_subsystem = "windows"]

mod theme;
mod hook;
mod conversions;
mod popup;
mod sender;

use std::cell::Cell;
use std::sync::Arc;
use std::time::Duration;
use bytes::Bytes;
use druid::widget::{Button, Flex, Label, SizedBox};
use druid::{AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, EventCtx, ExtEventSink, Handled, Selector, Target, Widget, WidgetExt, WindowDesc};
use druid::im::HashSet;
use quinn::{ClientConfig, Connection, Endpoint, TransportConfig};
use tokio::runtime::{Builder, Runtime};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use yawi::{HookAction, InputEvent, InputHook, KeyState, ScrollDirection, VirtualKey};
use serde::{Serialize, Deserialize};
use tokio::{select};
use tokio::time::{Instant};
use crate::conversions::{f32_to_i8, vk_to_mb, wsc_to_cdc, wsc_to_hkc};
use crate::hook::HookEvent;
use crate::popup::{Popup};
use crate::sender::InputSender;
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
    Connecting,
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
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting => "Connecting"
        }.to_string())
            .with_text_size(25.0))
        .with_spacer(20.0)
        .with_child(Button::from_label(Label::dynamic(|data: &AppState, _| match data.connection_state {
            ConnectionState::Connected(_) => "Disconnect",
            ConnectionState::Disconnected => "Connect",
            ConnectionState::Connecting => "Connecting"
        }.to_string())
            .with_text_size(17.0))
            .fix_size(250.0, 65.0)
            .on_click(|ctx, _, _| ctx.submit_command(CONNECT.with(())))
            .disabled_if(|data: &AppState, _ | data.connection_state == ConnectionState::Connecting))
        .with_default_spacer()
        .with_child(Button::dynamic(|data: &AppState, _|data.config.hotkey.trigger.to_string())
            .fix_width(250.0)
            .on_click(|ctx, _, _ |  open_key_picker(ctx,  |data, key| data.config.hotkey.trigger = key)))
        .center();
    let popup = Flex::column()
        .with_child(Label::new("Press any key"))
        .center();
    let popup = SizedBox::new(popup)
        .width(200.0)
        .height(100.0)
        .background(druid::theme::BACKGROUND_DARK)
        .rounded(5.0);
    Popup::new(|data: &AppState, _| data.popup, popup, ui)
}

fn open_key_picker(ctx: &mut EventCtx, setter: impl FnOnce(&mut AppState, VirtualKey) + Send + 'static) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| {
        if data.connection_state != ConnectionState::Disconnected {
            return;
        }
        assert!(rt.hook.is_none());
        let mut found = None;
        let setter = Cell::new(Some(setter));
        rt.hook = InputHook::register(move |event: InputEvent| {
            match  event.to_key_event() {
                Some(yawi::KeyEvent{key, state: KeyState::Pressed}) if found.is_none() || found == Some(key) => {
                    found = Some(key);
                    HookAction::Block
                },
                Some(yawi::KeyEvent{key, state: KeyState::Released}) if found == Some(key) => {
                    if let Some(setter) = setter.take() {
                        handle.add_rt_callback(move |rt, data| {
                            rt.hook = None;
                            setter(data, key);
                            data.popup = false;
                        });
                    }
                    HookAction::Block
                },
                _ => HookAction::Continue,
            }
        }).map_err(|err| tracing::warn!("Could not register hook: {}", err)).ok();
        data.popup = true;
    })
}

pub const CONNECT: Selector<()> = Selector::new("inputshare.connect");
type CallbackFunc = Cell<Option<Box<dyn FnOnce(&mut RuntimeDelegate, &mut AppState) + Send + 'static>>>;
const CALLBACK: Selector<CallbackFunc> = Selector::new("inputshare.callback");
trait ExtEventSinkCallback {
    fn add_rt_callback(self, callback: impl FnOnce(&mut RuntimeDelegate, &mut AppState) + Send + 'static);
}
impl ExtEventSinkCallback for &ExtEventSink {
    fn add_rt_callback(self, callback: impl FnOnce(&mut RuntimeDelegate, &mut AppState) + Send + 'static) {
        let callback: CallbackFunc = Cell::new(Some(Box::new(callback)));
        self.submit_command(CALLBACK, Box::new(callback), Target::Auto)
            .unwrap_or_else(|err| tracing::warn!("Could not submit callback: {}", err));
    }
}

impl ExtEventSinkCallback for &mut EventCtx<'_, '_> {
    fn add_rt_callback(self, callback: impl FnOnce(&mut RuntimeDelegate, &mut AppState) + Send + 'static) {
        let callback: CallbackFunc = Cell::new(Some(Box::new(callback)));
        self.submit_command(CALLBACK.with(callback));
    }
}

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
            cmd if cmd.is(CONNECT) => {
                self.hook = None;
                if std::mem::take(&mut data.connection_state) == ConnectionState::Disconnected{
                    let handle = ctx.get_external_handle();
                    data.connection_state = ConnectionState::Connecting;
                    self.runtime.spawn(async move {
                        connection(&handle)
                            .await
                            .unwrap_or_else(|err| tracing::warn!("could not establish connection: {}", err));
                        handle.add_rt_callback(|rt, data | {
                            rt.hook = None;
                            data.connection_state = ConnectionState::Disconnected;
                        });
                    });
                }
                Handled::Yes
            },
            cmd if cmd.is(CALLBACK) => {
                if let Some(callback) = cmd.get_unchecked(CALLBACK).take() {
                    callback(self, data);
                }
                Handled::Yes
            }
            _ => Handled::No
        }
    }
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

fn process_hook_event(sender: &mut InputSender, sink: &ExtEventSink, event: HookEvent) {
    match event {
        HookEvent::Captured(captured) => {
            sink.add_idle_callback(move |data: &mut AppState| {
                data.connection_state = ConnectionState::Connected(match captured {
                    true => Side::Remote,
                    false => Side::Local
                });
            });
            sender.reset();
        },
        HookEvent::Input(event) => match event {
            InputEvent::MouseMoveEvent(x, y) => {
                sender.move_mouse(x as i64, y as i64);
            }
            InputEvent::KeyboardKeyEvent(vk, sc, ks) => match wsc_to_hkc(sc) {
                Some(kc) => match ks {
                    KeyState::Pressed => sender.press_key(kc),
                    KeyState::Released => sender.release_key(kc)
                },
                None => match wsc_to_cdc(sc){
                    Some(cdc) => match ks {
                        KeyState::Pressed => sender.press_consumer_device(cdc),
                        KeyState::Released => sender.release_consumer_device(cdc)
                    },
                    None => if! matches!(sc, 0x21d) {
                        tracing::warn!("Unknown key: {} ({:x})", vk, sc)
                    }
                }
            }
            InputEvent::MouseButtonEvent(mb, ks) => match vk_to_mb(mb) {
                Some(button) => match ks {
                    KeyState::Pressed => sender.press_mouse_button(button),
                    KeyState::Released => sender.release_mouse_button(button)
                },
                None => tracing::warn!("Unknown mouse button: {}", mb)
            }
            InputEvent::MouseWheelEvent(sd) => match sd {
                ScrollDirection::Horizontal(amount) => sender.scroll_horizontal(f32_to_i8(amount)),
                ScrollDirection::Vertical(amount) => sender.scroll_vertical(f32_to_i8(amount))
            }
        }
    }
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