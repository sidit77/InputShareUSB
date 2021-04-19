extern crate native_windows_gui as nwg;
use std::ptr::null;
use std::os::raw;
use winapi::um::winuser::{SetWindowsHookExW, CallNextHookEx, WH_KEYBOARD_LL, MSG, TranslateMessage, DispatchMessageW, GetMessageW, UnhookWindowsHookEx};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::shared::windef::{HHOOK, POINT, HWND};
use winapi::shared::minwindef::{WPARAM, LPARAM, LRESULT, UINT, DWORD};
use std::rc::Rc;
use nwg::NativeUi;

const SERVER: &str = "127.0.0.1:12351";

static mut _hook: Option<HHOOK> = None;

unsafe extern "system" fn lpfn(code: raw::c_int, wParam: WPARAM, lParam: LPARAM) -> LRESULT {
    println!("{}", code);
    println!("{}", wParam);
    println!("{}", lParam);
    CallNextHookEx(_hook.unwrap(), code, wParam, lParam)
}

fn set_up_keyboard_hook() {
    unsafe {
        _hook = Some(SetWindowsHookExW(WH_KEYBOARD_LL, Some(lpfn), GetModuleHandleW(null()), 0));
    }
}

#[derive(Default)]
pub struct SystemTray {
    window: nwg::MessageWindow,
    icon: nwg::Icon,
    tray: nwg::TrayNotification,
    tray_menu: nwg::Menu,
    tray_item1: nwg::MenuItem,
    tray_item2: nwg::MenuItem,
    tray_item3: nwg::MenuItem,
}

impl SystemTray {

    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn hello1(&self) {
        nwg::modal_info_message(&self.window, "Hello", "Hello World!");
    }

    fn hello2(&self) {
        let flags = nwg::TrayNotificationFlags::USER_ICON | nwg::TrayNotificationFlags::LARGE_ICON;
        self.tray.show("Hello World", Some("Welcome to my application"), Some(flags), Some(&self.icon));
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

}


//
// ALL of this stuff is handled by native-windows-derive
//
mod system_tray_ui {
    use native_windows_gui as nwg;
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;
    use std::ops::Deref;

    pub struct SystemTrayUi {
        inner: Rc<SystemTray>,
        default_handler: RefCell<Vec<nwg::EventHandler>>
    }

    impl nwg::NativeUi<SystemTrayUi> for SystemTray {
        fn build_ui(mut data: SystemTray) -> Result<SystemTrayUi, nwg::NwgError> {
            use nwg::Event as E;

            // Resources
            nwg::Icon::builder()
                .source_file(Some("./test_rc/cog.ico"))
                .build(&mut data.icon)?;

            // Controls
            nwg::MessageWindow::builder()
                .build(&mut data.window)?;

            nwg::TrayNotification::builder()
                .parent(&data.window)
                .icon(Some(&data.icon))
                .tip(Some("Hello"))
                .build(&mut data.tray)?;

            nwg::Menu::builder()
                .popup(true)
                .parent(&data.window)
                .build(&mut data.tray_menu)?;

            nwg::MenuItem::builder()
                .text("Hello")
                .parent(&data.tray_menu)
                .build(&mut data.tray_item1)?;

            nwg::MenuItem::builder()
                .text("Popup")
                .parent(&data.tray_menu)
                .build(&mut data.tray_item2)?;

            nwg::MenuItem::builder()
                .text("Exit")
                .parent(&data.tray_menu)
                .build(&mut data.tray_item3)?;

            // Wrap-up
            let ui = SystemTrayUi {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };

            // Events
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data, handle| {
                if let Some(evt_ui) = evt_ui.upgrade() {
                    match evt {
                        E::OnContextMenu =>
                            if &handle == &evt_ui.tray {
                                SystemTray::show_menu(&evt_ui);
                            }
                        E::OnMenuItemSelected =>
                            if &handle == &evt_ui.tray_item1 {
                                SystemTray::hello1(&evt_ui);
                            } else if &handle == &evt_ui.tray_item2 {
                                SystemTray::hello2(&evt_ui);
                            } else if &handle == &evt_ui.tray_item3 {
                                SystemTray::exit(&evt_ui);
                            },
                        _ => {}
                    }
                }
            };

            ui.default_handler.borrow_mut().push(
                nwg::full_bind_event_handler(&ui.window.handle, handle_events)
            );

            return Ok(ui);
        }
    }

    impl Drop for SystemTrayUi {
        /// To make sure that everything is freed without issues, the default handler must be unbound.
        fn drop(&mut self) {
            let mut handlers = self.default_handler.borrow_mut();
            for handler in handlers.drain(0..) {
                nwg::unbind_event_handler(&handler);
            }
        }
    }

    impl Deref for SystemTrayUi {
        type Target = SystemTray;

        fn deref(&self) -> &SystemTray {
            &self.inner
        }
    }

}

fn main() {
    println!("Hello client!");

    unsafe {
        set_up_keyboard_hook();

        //let mut msg: MSG = MSG {
        //    hwnd : 0 as HWND,
        //    message : 0 as UINT,
        //    wParam : 0 as WPARAM,
        //    lParam : 0 as LPARAM,
        //    time : 0 as DWORD,
        //    pt : POINT { x: 0, y: 0, },
        //};
        //loop {
        //    let pm = GetMessageW(&mut msg, 0 as HWND, 0, 0);
        //    if pm == 0 {
        //        break;
        //    }
//
        //    TranslateMessage(&msg);
        //    DispatchMessageW(&msg);
        //}
    }

    nwg::init().expect("Failed to init Native Windows GUI");
    let _ui = SystemTray::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();

    unsafe {
        UnhookWindowsHookEx(_hook.unwrap());
    }

/*
    let addr = "127.0.0.1:12352";
    let mut socket = Socket::bind(addr).unwrap();
    println!("Connected on {}", addr);

    let server = SERVER.parse().unwrap();

    println!("Type a message and press Enter to send. Send `Bye!` to quit.");

    let stdin = stdin();
    let mut s_buffer = String::new();

    loop {
        s_buffer.clear();
        stdin.read_line(&mut s_buffer).unwrap();
        let line = s_buffer.replace(|x| x == '\n' || x == '\r', "");

        socket.send(Packet::reliable_unordered(
            server,
            line.clone().into_bytes(),
        )).unwrap();

        socket.manual_poll(Instant::now());

        if line == "Bye!" {
            break;
        }

        match socket.recv() {
            Some(SocketEvent::Packet(packet)) => {
                if packet.addr() == server {
                    println!("Server sent: {}", String::from_utf8_lossy(packet.payload()));
                } else {
                    println!("Unknown sender.");
                }
            }
            Some(SocketEvent::Timeout(_)) => {}
            _ => println!("Silence.."),
        }
    }

    */
}
