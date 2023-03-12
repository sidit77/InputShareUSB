use std::time::Instant;
use egui::Context;
use error_tools::log::LogResultExt;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;
use egui_d3d11::EguiD3D11;
use crate::windows::Direct3D;

pub struct EguiWindow {
    d3d: Direct3D,
    egui: EguiD3D11,
    window: Window,
    next_repaint: Option<Instant>
}

impl EguiWindow {

    pub fn new<T>(window: Window, event_loop: &EventLoopWindowTarget<T>) -> Self {
        let d3d = Direct3D::new(&window).expect("Can not create a directx context");
        let egui = EguiD3D11::new(event_loop, d3d.device.clone(), d3d.context.clone());
        Self {
            d3d,
            egui,
            window,
            next_repaint: Some(Instant::now()),
        }
    }

    pub fn ctx(&self) -> &Context {
        &self.egui.egui_ctx
    }

    pub fn handle_events<T>(&mut self, event: &Event<T>, gui: impl FnMut(&Context)) -> bool {
        if self.next_repaint().map(|t| Instant::now().checked_duration_since(t)).is_some() {
            self.window.request_redraw();
        }
        match event {
            Event::RedrawEventsCleared  => {
                let repaint_after = self.egui.run(&self.window, gui);
                self.next_repaint = Instant::now().checked_add(repaint_after);
                unsafe {
                    let render_target = self.d3d.render_target();
                    self.d3d.context.OMSetRenderTargets(Some(&[render_target]), None);
                    self.egui.paint(&self.window);
                    self.d3d.swap_chain
                        .Present(1, 0)
                        .ok()
                        .expect("Could not present swapchain");
                }
            },
            //Event::RedrawRequested(_) if !cfg!(windows) => self.redraw(gui),
            Event::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                        return true;
                    },
                    WindowEvent::Resized(size) => {
                        self.d3d.resize(size.width, size.height)
                            .log_ok("Could not resize swapchain");
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        let size = **new_inner_size;
                        self.d3d.resize(size.width, size.height)
                            .log_ok("Could not resize swapchain");
                    },
                    _ => {}
                }

                let event_response = self.egui.on_event(event);
                if event_response.repaint {
                    self.window.request_redraw();
                }
            }
            _ => (),
        }
        false
    }

    pub fn next_repaint(&self) -> Option<Instant> {
        self.next_repaint
    }

}

/*
mod system_menu;
mod key_tester;

use std::ptr::null_mut;
use system_menu::SystemMenu;
use native_windows_gui as nwg;
use native_windows_derive::NwgUi;
use native_windows_gui::{CharEffects, ControlHandle, MessageButtons, MessageIcons, MessageParams};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{MSG, PostMessageW, WM_SYSCOMMAND};

pub use key_tester::*;
use crate::CONNECT;


#[derive(Default, NwgUi)]
pub struct InputShareApp {
    #[nwg_resource(family: "Consolas", size: 12, weight: 500)]
    small_font: nwg::Font,

    #[nwg_control(size: (300, 133), position: (300, 300), title: "InputShare Client", flags: "WINDOW|VISIBLE|MINIMIZE_BOX",
    icon: Some(&nwg::EmbedResource::load(None)?.icon(1, None).unwrap()))]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,

    #[nwg_control()]
    system_menu: SystemMenu,

    #[nwg_control(parent: system_menu)]
    menu_separator: nwg::MenuSeparator,

    #[nwg_control(parent: system_menu, text: "Show Network Info", check: true)]
    network_info_toggle: nwg::MenuItem,

    #[nwg_control(parent: system_menu, text: "Open Key Tester")]
    //#[nwg_events( OnMenuItemSelected: [nwg::stop_thread_dispatch()] )]
    key_tester_button: nwg::MenuItem,

    #[nwg_control(parent: system_menu, text: "Shutdown Server", disabled: true)]
    shutdown_pi_button: nwg::MenuItem,

    #[nwg_control(text: "", font: Some(&data.small_font), size: (100, 13), position: (2, 2), flags: "VISIBLE")]
    info_label: nwg::Label,

    #[nwg_control(text: "Not Connected", size: (240, 45), position: (30, 10), flags: "VISIBLE|DISABLED")]
    status_label: nwg::RichLabel,

    #[nwg_control(text: "Connect", size: (280, 60), position: (10, 60))]
    #[nwg_events( OnButtonClick: [InputShareApp::connect_button_press] )]
    connect_button: nwg::Button,

}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GuiEvent {
    NetworkInfo,
    KeyTester,
    ShutdownServer
}

impl InputShareApp {

    pub fn handle_event(&self, msg: &MSG) -> Option<GuiEvent> {
        match self.window.handle.matches_hwnd(msg.hwnd) {
            true => match msg.message {
                WM_SYSCOMMAND => match msg.wParam as u32{
                    id if self.network_info_toggle.handle.matches_item_id(id) => Some(GuiEvent::NetworkInfo),
                    id if self.key_tester_button.handle.matches_item_id(id) => Some(GuiEvent::KeyTester),
                    id if self.shutdown_pi_button.handle.matches_item_id(id) => Some(GuiEvent::ShutdownServer),
                    _ => None
                },
                _ => None
            }
            false => None
        }
    }

    pub fn handle(&self) -> HWND {
        self.window.handle.hwnd().unwrap()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.window.set_enabled(enabled);
    }

    pub fn show_network_info_enabled(&self, enabled: bool) {
        self.network_info_toggle.set_checked(enabled);
        self.info_label.set_visible(enabled);
    }

    pub fn update_network_info(&self, info: &str) {
        self.info_label.set_text(info)
    }

    pub fn connect_button_press(&self) {
        unsafe { PostMessageW(null_mut(), CONNECT, 0, 0); }
    }

    pub fn set_connection_state(&self, state: ConnectionState) {
        self.connect_button.set_text(state.text());
        self.connect_button.set_enabled(matches!(state, ConnectionState::Connected | ConnectionState::Disconnected));
        self.key_tester_button.set_enabled(matches!(state, ConnectionState::Disconnected));
        self.shutdown_pi_button.set_enabled(matches!(state, ConnectionState::Connected));
    }

    pub fn set_status(&self, status: StatusText) {
        self.status_label.set_text(status.text());
        self.status_label.set_para_format(0..100, &nwg::ParaFormat {
            alignment: Some(nwg::ParaAlignment::Center),
            ..Default::default()
        });
        self.status_label.set_char_format(0..100, &nwg::CharFormat {
            height: Some(500),
            effects: Some(CharEffects::BOLD),
            text_color: Some(status.color()),
            //font_face_name: Some("Comic Sans MS".to_string()),
            ..Default::default()
        });
    }

    pub fn show_error(&self, msg: &str) {
        nwg::modal_message(&self.window, &MessageParams {
            title: "Error",
            content: msg,
            buttons: MessageButtons::Ok,
            icons: MessageIcons::Error
        });
    }

}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum StatusText {
    Local,
    Remote,
    NotConnected
}

impl StatusText {

    fn text(self) -> &'static str{
        match self {
            StatusText::Local => "Local",
            StatusText::Remote => "Remote",
            StatusText::NotConnected => "Not Connected",
        }
    }

    fn color(self) -> [u8; 3] {
        match self {
            StatusText::Local => [60, 140, 255],
            StatusText::Remote => [255, 80, 100],
            StatusText::NotConnected => [150, 150, 150],
        }
    }

}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnecting,
    Disconnected
}

impl ConnectionState {

    fn text(self) -> &'static str{
        match self {
            ConnectionState::Connecting => "Connecting...",
            ConnectionState::Connected => "Disconnect",
            ConnectionState::Disconnecting => "Disconnecting...",
            ConnectionState::Disconnected => "Connect"
        }
    }

}


trait ControlHandleExt {
    fn matches_hwnd(self, hwnd: HWND) -> bool;
    fn matches_item_id(self, id: u32) -> bool;
}

impl ControlHandleExt for ControlHandle {
    fn matches_hwnd(self, hwnd: HWND) -> bool {
        match self {
            ControlHandle::NoHandle => false,
            ControlHandle::Hwnd(v) => v == hwnd,
            ControlHandle::Menu(_, _) => false,
            ControlHandle::PopMenu(v, _) => v == hwnd,
            ControlHandle::MenuItem(_, _) => false,
            ControlHandle::Notice(v, _) => v == hwnd,
            ControlHandle::Timer(v, _) => v == hwnd,
            ControlHandle::SystemTray(v) => v == hwnd,
        }
    }

    fn matches_item_id(self, id: u32) -> bool {
        match self {
            ControlHandle::NoHandle => false,
            ControlHandle::Hwnd(_) => false,
            ControlHandle::Menu(_, _) => false,
            ControlHandle::PopMenu(_, _) => false,
            ControlHandle::MenuItem(_, v) => v == id,
            ControlHandle::Notice(_, v) => v == id,
            ControlHandle::Timer(_, v) => v == id,
            ControlHandle::SystemTray(_) => false
        }
    }
}
*/