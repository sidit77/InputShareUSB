use std::cell::Cell;

use druid::{AppDelegate, Command, DelegateCtx, Env, EventCtx, ExtEventSink, Handled, Selector, Target};
use mdns_sd::ServiceDaemon;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::UnboundedSender;
use yawi::InputHook;

use crate::model::{AppState, ConnectionCommand};

type CallbackFunc = Cell<Option<Box<dyn FnOnce(&mut RuntimeDelegate, &mut AppState) + Send + 'static>>>;
const CALLBACK: Selector<CallbackFunc> = Selector::new("inputshare.callback");

pub trait ExtEventSinkCallback {
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

pub struct RuntimeDelegate {
    pub runtime: Runtime,
    pub hook: Option<InputHook>,
    pub mdns: Option<ServiceDaemon>,
    pub connection: Option<UnboundedSender<ConnectionCommand>>
}

impl RuntimeDelegate {
    pub fn new() -> Self {
        Self {
            runtime: Builder::new_multi_thread()
                .enable_all()
                .worker_threads(1)
                .build()
                .expect("Could not start async runtime"),
            hook: None,
            mdns: None,
            connection: None
        }
    }
}

impl AppDelegate<AppState> for RuntimeDelegate {
    fn command(&mut self, _: &mut DelegateCtx, _target: Target, cmd: &Command, data: &mut AppState, _env: &Env) -> Handled {
        match cmd.get(CALLBACK) {
            Some(callback) => {
                if let Some(callback) = callback.take() {
                    callback(self, data);
                }
                Handled::Yes
            }
            None => Handled::No
        }
    }
}

impl Drop for RuntimeDelegate {
    fn drop(&mut self) {
        self.mdns = None;
        self.connection = None;
        self.hook = None;
        //self.runtime.shutdown_timeout(Duration::from_secs(10));
    }
}
