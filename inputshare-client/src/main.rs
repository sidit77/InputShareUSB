#![windows_subsystem = "windows"]

mod theme;

use druid::widget::{Button, Flex, Label};
use druid::{AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Handled, Selector, Target, Widget, WidgetExt, WindowDesc};
use error_tools::log::LogResultExt;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use yawi::{InputHook, KeyEvent, KeyState};
use crate::theme::Theme;

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

    /*
    let event_sink = launcher.get_external_handle();
    let _hook = InputHook::register(move |event| {
        if let Some(KeyEvent {key, state: KeyState::Pressed}) = event.to_key_event() {
            event_sink.add_idle_callback(move |data: &mut Key| {
                *data = Key(key);
            });
        }

        true
    }).unwrap();
     */

    launcher
        .delegate(NetworkHandler::new())
        .configure_env(|env, _| theme::setup_theme(Theme::Light, env))
        .launch(AppState::default())
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    Flex::column()
        .with_child(Label::dynamic(|data: &AppState, _| format!("{:?}", data.connection_state)))
        .with_child(Button::new("Change")
            .on_click(|ctx, _, _| ctx.submit_command(MSG.with(()))))
        .center()
}

pub const MSG: Selector<()> = Selector::new("inputshare.msg");

struct NetworkHandler {
    hook: Option<InputHook>
}

impl NetworkHandler {

    fn new() -> Self {
        Self {
            hook: None,
        }
    }

}

impl AppDelegate<AppState> for NetworkHandler {
    fn command(&mut self, _ctx: &mut DelegateCtx, _target: Target, cmd: &Command, data: &mut AppState, _env: &Env) -> Handled {
        if cmd.is(MSG) {
            self.hook = match self.hook.take() {
                None => InputHook::register(|event| {
                    if let Some(KeyEvent {key, state: KeyState::Pressed}) = event.to_key_event() {
                        tracing::info!("{:?}", key)
                    }
                    true
                }).log_ok("Failed to register hook"),
                Some(_) => None
            };
            data.connection_state = match self.hook.is_some() {
                true => ConnectionState::Connected,
                false => ConnectionState::Disconnected
            };
            Handled::Yes
        } else {
            Handled::No
        }
    }
}