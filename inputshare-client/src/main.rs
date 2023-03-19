#![windows_subsystem = "windows"]

mod theme;
mod hook;

use std::time::Duration;
use druid::widget::{Button, Flex, Label};
use druid::{AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Handled, Selector, Target, Widget, WidgetExt, WindowDesc};
use druid::im::HashSet;
use error_tools::log::LogResultExt;
use tokio::runtime::{Builder, Runtime};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use yawi::{InputHook, KeyEvent, KeyState, VirtualKey};
use serde::{Serialize, Deserialize};
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

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Data)]
enum ConnectionState {
    Connected,
    #[default]
    Disconnected
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Data)]
struct AppState {
    connection_state: ConnectionState
}

pub fn main() {
    tracing_subscriber::registry()
        .with(Targets::new()
            .with_default(LevelFilter::TRACE)
            .with_target("druid", LevelFilter::DEBUG))
        .with(layer()
            .without_time())
        .init();

    #[cfg(not(debug_assertions))]
    error_tools::gui::set_gui_panic_hook();

    let window = WindowDesc::new(make_ui())
        .window_size((400.0, 300.0))
        .title("InputShare Client");

    let launcher = AppLauncher::with_window(window);

    launcher
        .delegate(RuntimeDelegate::new())
        .configure_env(|env, _| theme::setup_theme(Theme::Light, env))
        .launch(AppState::default())
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    Flex::column()
        .with_child(Label::dynamic(|data: &AppState, _| format!("{:?}", data.connection_state)))
        .with_child(Button::dynamic(|data: &AppState, _| match data.connection_state {
            ConnectionState::Connected => "Disconnect",
            ConnectionState::Disconnected => "Connect"
        }.to_string())
            .on_click(|ctx, _, _| ctx.submit_command(MSG.with(()))))
        .center()
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
                self.hook = match self.hook.take() {
                    None => {
                        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
                        let hook = InputHook::register(move |event| {
                            if let Some(KeyEvent {key, state: KeyState::Pressed}) = event.to_key_event() {
                                sender.send(key)
                                    .log_ok("Could not send message to runtime");
                            }
                            true
                        }).log_ok("Failed to register hook");
                        if hook.is_some() {
                            self.runtime.spawn(async move {
                                while let Some(key) = receiver.recv().await {
                                    tracing::info!("New key: {:?}", key);
                                }
                                tracing::trace!("Ending async task");
                            });
                            let sink = ctx.get_external_handle();
                            self.runtime.spawn(async move {
                                tokio::time::sleep(Duration::from_secs(3)).await;
                                tracing::trace!("removing hook");
                                sink.submit_command(RESET, (), Target::Auto)
                                    .log_ok("Failed to submit reset command");
                            });
                        }
                        hook
                    },
                    Some(_) => None
                };
                data.connection_state = match self.hook.is_some() {
                    true => ConnectionState::Connected,
                    false => ConnectionState::Disconnected
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