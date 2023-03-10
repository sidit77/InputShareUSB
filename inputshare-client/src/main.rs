#![windows_subsystem = "windows"]

mod windows;

use std::time::Instant;
use anyhow::Result;
use egui::Visuals;
use error_tools::log::LogResultExt;
use log::LevelFilter;
use winit::dpi::PhysicalSize;
use winit::event::{Event, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::platform::windows::{IconExtWindows, WindowBuilderExtWindows};
use winit::window::{Icon, WindowBuilder};
use egui_d3d11::EguiD3D11;
use crate::windows::{com_initialized, Direct3D};

fn main() -> Result<()>{
    error_tools::gui::set_gui_panic_hook();

    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .format_timestamp(None)
        //.format_target(false)
        .parse_default_env()
        .init();

    com_initialized();

    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("InputShare Client")
        .with_drag_and_drop(false)
        .with_window_icon(Some(Icon::from_resource(32512, None)?))
        .with_inner_size(PhysicalSize::new(400, 300))
        .build(&event_loop)?;

    let mut d3d = Direct3D::new(&window)?;
    let mut egui_d3d = EguiD3D11::new(&event_loop, d3d.device.clone(), d3d.context.clone());
    egui_d3d.egui_ctx.set_visuals(Visuals::light());

    event_loop.run_return(move |event, _, control_flow| {
        let mut redraw = || {

            let repaint_after = egui_d3d.run(&window, |egui_ctx| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.heading("Hello world");
                    })
                });
            });

            *control_flow = if repaint_after.is_zero() {
                window.request_redraw();
                ControlFlow::Poll
            } else if let Some(repaint_after_instant) = Instant::now().checked_add(repaint_after) {
                ControlFlow::WaitUntil(repaint_after_instant)
            } else {
                ControlFlow::Wait
            };

            {
                unsafe {
                    d3d.context.OMSetRenderTargets(Some(&[d3d.render_target().clone()]), None);
                }

                egui_d3d.paint(&window);

                unsafe {
                    d3d.swap_chain.Present(1, 0)
                        .ok().unwrap();
                }
            }
        };

        match event {
            Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            Event::WindowEvent { event, .. } => {
                if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                    *control_flow = ControlFlow::Exit;
                }
                if let WindowEvent::Resized(size) = &event {
                    d3d.resize(size.width, size.height)
                        .log_ok("Can not resize resources");
                    //log::trace!("Resized dx resources to {}/{}", size.width, size.height);
                } else if let WindowEvent::ScaleFactorChanged { .. } = &event {
                    log::error!("Need to resize");
                }

                let event_response = egui_d3d.on_event(&event);
                if event_response.repaint {
                    window.request_redraw();
                }
            }
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => window.request_redraw(),
            _ => (),
        }
    });
    Ok(())
}