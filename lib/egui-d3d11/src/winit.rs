use std::time::Duration;
use egui::{Context, TexturesDelta};
use egui::epaint::ClippedShape;
use egui_winit::{EventResponse, State};
use egui_winit::winit::event_loop::EventLoopWindowTarget;
use egui_winit::winit::event::WindowEvent;
use egui_winit::winit::window::Window;
use crate::{Device, DeviceContext};
use crate::painter::Painter;

pub struct EguiD3D11 {
    pub egui_ctx: Context,
    pub egui_winit: State,
    pub painter: Painter,

    shapes: Vec<ClippedShape>,
    textures_delta: TexturesDelta
}

impl EguiD3D11 {

    pub fn new<E>(event_loop:  &EventLoopWindowTarget<E>, device: impl Into<Device>, context: impl Into<DeviceContext>) -> Self {
        let painter = Painter::new(device, context);

        Self {
            egui_ctx: Default::default(),
            egui_winit: State::new(event_loop),
            painter,
            shapes: Vec::new(),
            textures_delta: TexturesDelta::default(),
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent<'_>) -> EventResponse {
        self.egui_winit.on_event(&self.egui_ctx, event)
    }

    pub fn run(&mut self, window: &Window, run_ui: impl FnMut(&Context), ) -> Duration {
        let raw_input = self.egui_winit.take_egui_input(window);
        let egui::FullOutput {
            platform_output,
            repaint_after,
            textures_delta,
            shapes,
        } = self.egui_ctx.run(raw_input, run_ui);

        self.egui_winit
            .handle_platform_output(window, &self.egui_ctx, platform_output);

        self.shapes = shapes;
        self.textures_delta.append(textures_delta);
        repaint_after
    }

    pub fn paint(&mut self, window: &Window) {
        let shapes = std::mem::take(&mut self.shapes);
        let mut textures_delta = std::mem::take(&mut self.textures_delta);

        for (id, image_delta) in textures_delta.set {
            self.painter.set_texture(id, &image_delta);
        }

        let clipped_primitives = self.egui_ctx.tessellate(shapes);
        let dimensions: [u32; 2] = window.inner_size().into();
        self.painter.paint_primitives(dimensions, self.egui_ctx.pixels_per_point(), &clipped_primitives);

        for id in textures_delta.free.drain(..) {
            self.painter.free_texture(id);
        }
    }
}