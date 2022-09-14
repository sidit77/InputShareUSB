use std::collections::HashSet;
use yawi::{InputEvent, InputHook, KeyState};
use anyhow::Result;
use native_windows_gui as nwg;
use native_windows_derive::NwgUi;
use native_windows_gui::NativeUi;
use crate::wsc_to_hkc;

#[derive(Default, NwgUi)]
pub struct KeyTester {
    #[nwg_resource(family: "Consolas", size: 25, weight: 500)]
    small_font: nwg::Font,

    #[nwg_control(size: (300, 50), position: (300, 300), title: "Key Tester", flags: "WINDOW|VISIBLE",
    icon: Some(&nwg::EmbedResource::load(None)?.icon(1, None).unwrap()))]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()])]
    window: nwg::Window,

    #[nwg_control(text: "Press a key", h_align: HTextAlign::Center, v_align: VTextAlign::Center,
    font: Some(&data.small_font), size: (300, 50), position: (0, 10), flags: "VISIBLE")]
    info_label: nwg::Label,

}

pub fn run_key_tester() -> Result<()> {
    let app = KeyTester::build_ui(Default::default())?;
    let mut pressed_keys = HashSet::new();
    let _h = InputHook::register(move |event|{
        if let Some(event) = event.to_key_event() {
            match (pressed_keys.contains(&event.key), event.state) {
                (false, KeyState::Pressed) => {
                    pressed_keys.insert(event.key);
                    app.info_label.set_text(&format!("{:?}", event.key))
                },
                (true, KeyState::Released) => {
                    pressed_keys.remove(&event.key);
                },
                _ => { }
            }
        }
        if let InputEvent::KeyboardKeyEvent(_, sc, _) = event {
            log::info!("{:?} {:?}", event, wsc_to_hkc(sc));
        }
        true
    });
    nwg::dispatch_thread_events();
    Ok(())
}