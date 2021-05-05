use yawi::{Input, VirtualKey, KeyState};
use std::thread;
use std::time::Duration;

fn main() {

    yawi::send_inputs(&[
        Input::KeyboardKeyInput(VirtualKey::LWin, KeyState::Pressed),
        Input::KeyboardKeyInput(VirtualKey::LWin, KeyState::Released),
    ]);

    thread::sleep(Duration::from_millis(1000));

    yawi::send_input(Input::StringInput("Notepad"));

    thread::sleep(Duration::from_millis(1000));

    yawi::send_inputs(&[
        Input::KeyboardKeyInput(VirtualKey::Return, KeyState::Pressed),
        Input::KeyboardKeyInput(VirtualKey::Return, KeyState::Released),
    ]);

    thread::sleep(Duration::from_millis(1000));

    yawi::send_input(Input::StringInput("Hello World!"));
}