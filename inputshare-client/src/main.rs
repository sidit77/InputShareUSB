#![windows_subsystem = "windows"]

mod windows;
mod ui;

use std::time::Instant;
use anyhow::Result;
use egui::{Visuals};
use log::LevelFilter;
use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::platform::windows::{IconExtWindows, WindowBuilderExtWindows};
use winit::window::{Icon, WindowBuilder};
use crate::ui::EguiWindow;
use crate::windows::{com_initialized};

fn main() -> Result<()>{
    env_logger::builder()
        .filter_level(LevelFilter::Trace)
        .format_timestamp(None)
        //.format_target(false)
        .parse_default_env()
        .init();

    #[cfg(not(debug_assertions))]
    error_tools::gui::set_gui_panic_hook();
    com_initialized();

    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("InputShare Client")
        .with_drag_and_drop(false)
        .with_window_icon(Some(Icon::from_resource(32512, None)?))
        .with_inner_size(PhysicalSize::new(400, 300))
        .build(&event_loop)?;

    let mut window = EguiWindow::new(window, &event_loop);
    window.ctx().set_visuals(Visuals::light());

    event_loop.run_return(move |event, _, control_flow| {
        let close_requested = window.handle_events(&event, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Hello world");
                        ui.spinner();
                    });
                });
            });
        });
        if close_requested {
            *control_flow = ControlFlow::Exit;
        }
        if !matches!(*control_flow, ControlFlow::ExitWithCode(_)) {
            let next_update = window.next_repaint();
            *control_flow = match next_update {
                None => ControlFlow::Wait,
                Some(deadline) if deadline <= Instant::now() => ControlFlow::Poll,
                Some(deadline) => ControlFlow::WaitUntil(deadline)
            }
        }
    });
    Ok(())
}

