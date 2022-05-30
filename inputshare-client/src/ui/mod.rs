mod system_menu;
mod key_tester;

use std::ptr::null_mut;
use system_menu::SystemMenu;
use native_windows_gui as nwg;
use native_windows_derive::NwgUi;
use native_windows_gui::{CharEffects, MessageButtons, MessageIcons, MessageParams};
use winapi::um::winuser::PostMessageW;

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
    key_tester_button: nwg::MenuItem,

    #[nwg_control(parent: system_menu, text: "Shutdown Server", disabled: true)]
    shutdown_pi_button: nwg::MenuItem,

    #[nwg_control(text: "", font: Some(&data.small_font), size: (100, 13), position: (2, 2), flags: "VISIBLE")]
    pub info_label: nwg::Label,

    #[nwg_control(text: "Not Connected", size: (240, 45), position: (30, 10), flags: "VISIBLE|DISABLED")]
    pub status_label: nwg::RichLabel,

    #[nwg_control(text: "Connect", size: (280, 60), position: (10, 60))]
    #[nwg_events( OnButtonClick: [InputShareApp::connect] )]
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

impl InputShareApp {

    pub fn connect(&self) {
        //nwg::simple_message("Hello", &format!("Hello {}", self.name_edit.text()));
        unsafe {
            PostMessageW(null_mut(), CONNECT, 0, 0);
        }
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