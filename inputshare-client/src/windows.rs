use std::marker::PhantomData;
use anyhow::Result;
use error_tools::SomeOptionExt;
use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::System::Com::{CoUninitialize, CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use winit::platform::windows::WindowExtWindows;
use winit::window::Window;

pub struct Direct3D {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext4,
    pub swap_chain: IDXGISwapChain1,
    render_target: Option<ID3D11RenderTargetView>
}

impl Direct3D {

    pub fn new(window: &Window) -> Result<Self> {
        let mut d3d_device = None;
        let mut d3d_ctx = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&[D3D_FEATURE_LEVEL_11_1]),
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                None,
                Some(&mut d3d_ctx),
            )?;
        }
        let d3d_device = d3d_device.some()?;
        let d3d_ctx = d3d_ctx.some()?.cast::<ID3D11DeviceContext4>()?;

        let dxgi_factory = unsafe { CreateDXGIFactory1::<IDXGIFactory2>()? };
        let window_size = window.inner_size();
        let swap_chain = unsafe {
            dxgi_factory.CreateSwapChainForHwnd(
                &d3d_device,
                HWND(window.hwnd() as _),
                &DXGI_SWAP_CHAIN_DESC1 {
                    Width: window_size.width,
                    Height: window_size.height,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    BufferCount: 2,
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                    Scaling: DXGI_SCALING_NONE,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    ..Default::default()
                },
                None,
                None,
            )?
        };

        let rtv = unsafe {
            let buffer = swap_chain.GetBuffer::<ID3D11Texture2D>(0)?;
            let mut target = std::mem::zeroed();
            d3d_device.CreateRenderTargetView(&buffer, None, Some(&mut target))?;
            target.some()?
        };

        Ok(Self {
            device: d3d_device,
            context: d3d_ctx,
            swap_chain,
            render_target: Some(rtv),
        })
    }

    pub fn render_target(&self) -> &ID3D11RenderTargetView {
        self.render_target.as_ref()
            .expect("The rendertarget should never not be initialized")
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        unsafe {
            self.context.OMSetRenderTargets(None, None);
            self.render_target = None;
            self.swap_chain.ResizeBuffers(0, width, height, DXGI_FORMAT_UNKNOWN, 0)?;
            let buffer = self.swap_chain.GetBuffer::<ID3D11Texture2D>(0)?;
            self.device.CreateRenderTargetView(&buffer, None, Some(&mut self.render_target))?;
            Ok(())
        }
    }

}

#[derive(Default)]
struct ComWrapper {
    _ptr: PhantomData<*mut ()>,
}

thread_local!(static COM_INITIALIZED: ComWrapper = {
    unsafe {
        SetProcessDpiAwarenessContext(Some(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2));
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .expect("Could not initialize COM");
        let thread = std::thread::current();
        log::trace!("Initialized COM on thread \"{}\"", thread.name().unwrap_or(""));
        ComWrapper::default()
    }
});

impl Drop for ComWrapper {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
            let thread = std::thread::current();
            log::trace!("Uninitialized COM on thread \"{}\"", thread.name().unwrap_or(""));
        }
    }
}

#[inline]
pub fn com_initialized() {
    COM_INITIALIZED.with(|_| {});
}

/*
use std::convert::TryInto;
use std::mem;
use std::ptr::{null, null_mut};
use std::time::Duration;
use winapi::shared::minwindef::{FALSE};
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0, WAIT_FAILED};
use winapi::um::winuser::{DispatchMessageW, GA_ROOT, GetAncestor, IsDialogMessageW, MSG, MsgWaitForMultipleObjects, PeekMessageW, PM_REMOVE, QS_ALLINPUT, TranslateMessage};


pub fn wait_message_timeout(timeout: Option<Duration>) -> std::io::Result<bool> {
    let timeout = match timeout {
        None => INFINITE,
        Some(duration) => duration.as_millis().try_into().expect("timout to large")
    };
    unsafe {
        match MsgWaitForMultipleObjects(0, null(), FALSE, timeout, QS_ALLINPUT) {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            WAIT_FAILED => Err(std::io::Error::last_os_error()),
            _ => panic!("invalid return type")
        }
    }
}

pub fn get_message() -> Option<MSG> {
    unsafe {
        let mut msg: MSG = mem::zeroed();
        match PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) {
            FALSE => None,
            _ => {
                if IsDialogMessageW(GetAncestor(msg.hwnd, GA_ROOT), &mut msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                Some(msg)
            }
        }
    }
}
*/