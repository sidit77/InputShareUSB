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
    pub window: nwg::Window,

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
    pub status_label: nwg::RichLabel,

    #[nwg_control(text: "Connect", size: (280, 60), position: (10, 60))]
    #[nwg_events( OnButtonClick: [InputShareApp::connect_button_press] )]
    pub connect_button: nwg::Button,


}

#[derive(Debug, Copy, Clone)]
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