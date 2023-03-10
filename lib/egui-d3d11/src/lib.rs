use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext};

mod painter;

#[cfg(feature = "egui-winit")]
mod winit;

pub type Device = ID3D11Device;
pub type DeviceContext = ID3D11DeviceContext;

pub use painter::{Painter, CallbackFn};
#[cfg(feature = "egui-winit")]
pub use winit::EguiD3D11;
