#![windows_subsystem = "windows"]

use druid::widget::{Label};
use druid::{AppLauncher, Widget, WidgetExt, WindowDesc};
use yawi::InputHook;

pub fn main() {
    #[cfg(not(debug_assertions))]
    error_tools::gui::set_gui_panic_hook();

    let window = WindowDesc::new(make_ui())
        .window_size((400.0, 300.0))
        .title("InputShare Client");

    let launcher = AppLauncher::with_window(window);

    let event_sink = launcher.get_external_handle();
    let _hook = InputHook::register(move |event| {
        event_sink.add_idle_callback(move |data: &mut String| {
            *data = format!("{:?}", event);
        });
        true
    }).unwrap();

    launcher
        .log_to_console()
        .launch(String::from("None"))
        .expect("launch failed");
}

fn make_ui() -> impl Widget<String> {
    Label::dynamic(|data: &String, _| data.clone())
        .center()
}