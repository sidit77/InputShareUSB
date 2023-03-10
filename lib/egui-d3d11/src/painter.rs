use std::collections::HashMap;
use std::io::Write;
use std::mem::size_of;
use bytemuck::Pod;
use egui::epaint::{ImageDelta, Primitive, Vertex};
use egui::{ClippedPrimitive, Mesh, PaintCallbackInfo, Rect, TextureFilter, TextureId, TextureOptions};
use windows::Win32::Foundation::{FALSE, RECT, TRUE};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST;
use windows::Win32::Graphics::Dxgi::Common::*;
use crate::{Device, DeviceContext};

pub struct CallbackFn {
    f: Box<dyn Fn(PaintCallbackInfo, &Painter) + Sync + Send>,
}

impl CallbackFn {
    pub fn new<F: Fn(PaintCallbackInfo, &Painter) + Sync + Send + 'static>(callback: F) -> Self {
        let f = Box::new(callback);
        CallbackFn { f }
    }
}

pub struct Painter {
    device: Device,
    context: DeviceContext,

    rasterizer_state: ID3D11RasterizerState,
    blend_state: ID3D11BlendState,
    vertex_buffer: ID3D11Buffer,
    index_buffer: ID3D11Buffer,
    vertex_shader: ID3D11VertexShader,
    pixel_shader: ID3D11PixelShader,
    input_layout: ID3D11InputLayout,
    constant_buffer: ID3D11Buffer,

    textures: HashMap<TextureId, Texture>
}

impl Painter {

    pub fn new(device: impl Into<Device>, context: impl Into<DeviceContext>) -> Self {
        let device = device.into();
        let context = context.into();
        let rasterizer_state = make_resource(|ptr| unsafe {
            device.CreateRasterizerState(&D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
                CullMode: D3D11_CULL_NONE,
                FrontCounterClockwise: FALSE,
                DepthBias: 0,
                DepthBiasClamp: 0.0,
                SlopeScaledDepthBias: 0.0,
                DepthClipEnable: TRUE,
                ScissorEnable: TRUE,
                MultisampleEnable: FALSE,
                AntialiasedLineEnable: FALSE,
            }, ptr)
        });
        let blend_state = make_resource(|ptr| unsafe {
            device.CreateBlendState(&D3D11_BLEND_DESC {
                AlphaToCoverageEnable: FALSE,
                IndependentBlendEnable: FALSE,
                RenderTarget: [D3D11_RENDER_TARGET_BLEND_DESC {
                    BlendEnable: TRUE,
                    SrcBlend: D3D11_BLEND_ONE,
                    DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                    BlendOp: D3D11_BLEND_OP_ADD,
                    SrcBlendAlpha: D3D11_BLEND_INV_DEST_ALPHA,
                    DestBlendAlpha:  D3D11_BLEND_ONE,
                    BlendOpAlpha: D3D11_BLEND_OP_ADD,
                    RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as _,
                }; 8],
            }, ptr)
        });
        let vertex_buffer = make_resource(|ptr| unsafe {
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: (200 * size_of::<Vertex>()) as u32,
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_VERTEX_BUFFER,
                    CPUAccessFlags:  D3D11_CPU_ACCESS_WRITE,
                    ..Default::default()
                }, None, ptr)
        });
        let index_buffer = make_resource(|ptr| unsafe {
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: (200 * size_of::<u32>()) as u32,
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_INDEX_BUFFER,
                    CPUAccessFlags:  D3D11_CPU_ACCESS_WRITE,
                    ..Default::default()
                }, None, ptr)
        });
        let constant_buffer = make_resource(|ptr| unsafe {
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: (size_of::<[f32;4]>()) as u32,
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                    CPUAccessFlags:  D3D11_CPU_ACCESS_WRITE,
                    ..Default::default()
                }, None, ptr)
        });
        let vs_blob = include_bytes!(concat!(env!("OUT_DIR"), "/shader.vs_blob"));
        let ps_blob = include_bytes!(concat!(env!("OUT_DIR"), "/shader.ps_blob"));
        let vertex_shader = make_resource(|ptr| unsafe {
            device.CreateVertexShader(vs_blob, None, ptr)
        });
        let pixel_shader = make_resource(|ptr| unsafe {
            device.CreatePixelShader(ps_blob, None, ptr)
        });
        let input_layout = make_resource(|ptr| unsafe {
            device.CreateInputLayout(&[
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: windows::s!("POSITION"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 0,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: windows::s!("TEXCOORD"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: windows::s!("COLOR"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    InputSlot: 0,
                    AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
            ], vs_blob, ptr)
        });
        Self {
            device,
            context,
            rasterizer_state,
            blend_state,
            vertex_buffer,
            index_buffer,
            vertex_shader,
            pixel_shader,
            input_layout,
            constant_buffer,
            textures: Default::default(),
        }
    }

    fn update_buffer<T: Pod>(device: &Device, context: &DeviceContext, buffer: &mut ID3D11Buffer, data: &[T]) {
        unsafe {
            let required_data = (data.len() * size_of::<T>()) as u32;
            let mut desc = retrieve(buffer, ID3D11Buffer::GetDesc);
            if desc.ByteWidth < required_data {
                desc.ByteWidth = required_data;
                *buffer = make_resource(|ptr| {
                    device.CreateBuffer(&desc, None, ptr)
                });
            }
            let buffer: &ID3D11Buffer = buffer;

            let mut slice = {
                let mut resource = std::mem::MaybeUninit::zeroed();
                context.Map(buffer, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(resource.as_mut_ptr()))
                    .expect("Can not map buffer");
                std::slice::from_raw_parts_mut(resource.assume_init().pData as *mut u8, desc.ByteWidth as usize)
            };
            slice.write_all(bytemuck::cast_slice(data))
                .expect("Can not update buffer data");
            context.Unmap(buffer, 0);
        }
    }

    pub fn paint_primitives(&mut self, screen_size_px: [u32; 2], pixels_per_point: f32, clipped_primitives: &[ClippedPrimitive]) {
        let size_in_pixels = unsafe { self.prepare_painting(screen_size_px, pixels_per_point) };

        for ClippedPrimitive { clip_rect, primitive, } in clipped_primitives {
            set_clip_rect(&self.context, size_in_pixels, pixels_per_point, *clip_rect);

            match primitive {
                Primitive::Mesh(mesh) => {
                    self.paint_mesh(mesh);
                }
                Primitive::Callback(callback) => {
                    if callback.rect.is_positive() {
                        // Transform callback rect to physical pixels:
                        let rect_min_x = pixels_per_point * callback.rect.min.x;
                        let rect_min_y = pixels_per_point * callback.rect.min.y;
                        let rect_max_x = pixels_per_point * callback.rect.max.x;
                        let rect_max_y = pixels_per_point * callback.rect.max.y;

                        let rect_min_x = rect_min_x.round() as i32;
                        let rect_min_y = rect_min_y.round() as i32;
                        let rect_max_x = rect_max_x.round() as i32;
                        let rect_max_y = rect_max_y.round() as i32;

                        unsafe {
                            self.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                                TopLeftX: rect_min_x as f32,
                                TopLeftY: (size_in_pixels.1 as i32 - rect_max_y) as f32,
                                Width: (rect_max_x - rect_min_x) as f32,
                                Height: (rect_max_y - rect_min_y) as f32,
                                MinDepth: 0.0,
                                MaxDepth: 1.0,
                            }]));
                        }

                        let info = PaintCallbackInfo {
                            viewport: callback.rect,
                            clip_rect: *clip_rect,
                            pixels_per_point,
                            screen_size_px,
                        };

                        if let Some(callback) = callback.callback.downcast_ref::<CallbackFn>() {
                            (callback.f)(info, self);
                        } else {
                            log::warn!("Warning: Unsupported render callback. Expected egui_glow::CallbackFn");
                        }

                        //check_for_gl_error!(&self.gl, "callback");

                        // Restore state:
                        unsafe { self.prepare_painting(screen_size_px, pixels_per_point) };
                    }
                }
            }
        }
    }

    fn paint_mesh(&mut self, mesh: &Mesh) {
        debug_assert!(mesh.is_valid());
        if let Some(texture) = self.textures.get(&mesh.texture_id) {
            Self::update_buffer(&self.device, &self.context, &mut self.vertex_buffer, &mesh.vertices);
            Self::update_buffer(&self.device, &self.context, &mut self.index_buffer, &mesh.indices);

            unsafe {
                self.context.IASetIndexBuffer(&self.index_buffer, DXGI_FORMAT_R32_UINT, 0);
                self.context.IASetVertexBuffers(
                    0,
                    1,
                    Some(&Some(self.vertex_buffer.clone())),
                    Some(&(size_of::<Vertex>() as u32)),
                    Some(&0)
                );
                self.context.PSSetSamplers(0,Some(&[texture.sampler.clone()]));
                self.context.PSSetShaderResources(0, Some(&[texture.view.clone()]));
                self.context.DrawIndexed(mesh.indices.len() as u32, 0 ,0);
            }

        } else {
            log::warn!("Failed to find texture {:?}", mesh.texture_id);
        }
    }

    unsafe fn prepare_painting(&mut self, [width_in_pixels, height_in_pixels]: [u32; 2], pixels_per_point: f32, ) -> (u32, u32) {
        self.context.RSSetState(&self.rasterizer_state);
        self.context.OMSetBlendState(&self.blend_state, None, u32::MAX);

        self.context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        self.context.IASetInputLayout(&self.input_layout);
        self.context.VSSetShader(&self.vertex_shader, None);
        self.context.PSSetShader(&self.pixel_shader, None);

        let width_in_points = width_in_pixels as f32 / pixels_per_point;
        let height_in_points = height_in_pixels as f32 / pixels_per_point;

        self.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: width_in_pixels as f32,
            Height: height_in_pixels as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        }]));

        Self::update_buffer(&self.device, &self.context, &mut self.constant_buffer, &[width_in_points, height_in_points]);
        self.context.VSSetConstantBuffers(0, Some(&[
            self.constant_buffer.clone()
        ]));

        (width_in_pixels, height_in_pixels)
    }

    pub fn set_texture(&mut self, tex_id: TextureId, delta: &ImageDelta) {
        if !self.textures.contains_key(&tex_id) {
            let tex = Texture::new(&self.device, delta.image.size(), delta.options);
            self.textures.insert(tex_id, tex);
        }
        let texture = self.textures.get_mut(&tex_id).unwrap();

        match &delta.image {
            egui::ImageData::Color(image) => {
                assert_eq!(image.width() * image.height(), image.pixels.len(),
                    "Mismatch between texture size and texel count");

                let data: &[u8] = bytemuck::cast_slice(image.pixels.as_ref());
                texture.upload_texture_srgb(&self.device, &self.context, delta.pos, image.size, delta.options, data);
            }
            egui::ImageData::Font(image) => {
                assert_eq!(image.width() * image.height(), image.pixels.len(),
                    "Mismatch between texture size and texel count");

                let data: Vec<u8> = image
                    .srgba_pixels(None)
                    .flat_map(|a| a.to_array())
                    .collect();

                texture.upload_texture_srgb(&self.device, &self.context, delta.pos, image.size, delta.options, &data);
            }
        }
    }

    pub fn free_texture(&mut self, tex_id: TextureId) {
        self.textures.remove(&tex_id);
    }

}

fn set_clip_rect(context:  &DeviceContext, size_in_pixels: (u32, u32), pixels_per_point: f32, clip_rect: Rect) {
    // Transform clip rect to physical pixels:
    let clip_min_x = pixels_per_point * clip_rect.min.x;
    let clip_min_y = pixels_per_point * clip_rect.min.y;
    let clip_max_x = pixels_per_point * clip_rect.max.x;
    let clip_max_y = pixels_per_point * clip_rect.max.y;

    // Round to integer:
    let clip_min_x = clip_min_x.round() as i32;
    let clip_min_y = clip_min_y.round() as i32;
    let clip_max_x = clip_max_x.round() as i32;
    let clip_max_y = clip_max_y.round() as i32;

    // Clamp:
    let clip_min_x = clip_min_x.clamp(0, size_in_pixels.0 as i32);
    let clip_min_y = clip_min_y.clamp(0, size_in_pixels.1 as i32);
    let clip_max_x = clip_max_x.clamp(clip_min_x, size_in_pixels.0 as i32);
    let clip_max_y = clip_max_y.clamp(clip_min_y, size_in_pixels.1 as i32);

    unsafe {
        context.RSSetScissorRects(Some(&[
            RECT {
                left: clip_min_x,
                top: clip_min_y,
                right: clip_max_x,
                bottom: clip_max_y,
            }
        ]))
    }

}

struct Texture {
    texture: ID3D11Texture2D,
    view: ID3D11ShaderResourceView,
    sampler: ID3D11SamplerState,
}

impl Texture {
    fn new(device: &Device, [width, height]: [usize; 2], options: TextureOptions) -> Self {
        let texture = make_resource(|ptr| unsafe {
            device.CreateTexture2D(&D3D11_TEXTURE2D_DESC {
                Width: width as u32,
                Height: height as u32,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET,
                CPUAccessFlags: D3D11_CPU_ACCESS_FLAG(0),
                MiscFlags: D3D11_RESOURCE_MISC_FLAG(0),
            }, None, ptr)
        });
        let view = make_resource(|ptr| unsafe {
            device.CreateShaderResourceView(&texture, None, ptr)
        });
        let sampler = make_resource(|ptr| unsafe {
            device.CreateSamplerState(&D3D11_SAMPLER_DESC {
                Filter: make_filter(options),
                AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                MipLODBias: 0.0,
                MaxAnisotropy: 1,
                MinLOD: f32::MIN,
                MaxLOD: f32::MAX,
                ..Default::default()
            }, ptr)
        });
        Self {
            texture,
            view,
            sampler,
        }
    }

    fn upload_texture_srgb(&mut self, device: &Device, context: &DeviceContext,
                           pos: Option<[usize; 2]>, [w, h]: [usize; 2], options: TextureOptions, data: &[u8]) {
        let max_size = D3D11_REQ_TEXTURE2D_U_OR_V_DIMENSION as usize;
        assert_eq!(data.len(), w * h * 4);
        assert!(w <= max_size && h <= max_size,
                "Got a texture image of size {}x{}, but the maximum supported texture side is only {}",
                w, h, max_size);
        unsafe {
            let filter = make_filter(options);
            let mut desc = retrieve(&self.sampler, ID3D11SamplerState::GetDesc);
            if desc.Filter != filter {
                desc.Filter = filter;
                self.sampler = make_resource(|ptr| {
                    device.CreateSamplerState(&desc, ptr)
                });
            }
        }
        unsafe {
            let mut desc = retrieve(&self.texture, ID3D11Texture2D::GetDesc);
            match pos {
                None => if desc.Width != w as u32 || desc.Height != h as u32 {
                    desc.Width = w as u32;
                    desc.Height = h as u32;
                    self.texture = make_resource(|ptr| {
                        device.CreateTexture2D(&desc, None, ptr)
                    });
                    self.view = make_resource(|ptr| {
                        device.CreateShaderResourceView(&self.texture, None, ptr)
                    });
                }
                Some([x, y]) => assert!(x + w <= desc.Width as usize && y + h <= desc.Height as usize)
            }
        }
        unsafe {
            let level = 0;
            let [x, y] = pos.unwrap_or([0, 0]);
            context.UpdateSubresource(
                &self.texture,
                level,
                Some(&make_box(x,y, w, h)),
                data.as_ptr() as _,
                (4 * w) as u32,
                (4 * w * h) as u32);
        }
    }
}

fn make_filter(options: TextureOptions) -> D3D11_FILTER {
    use TextureFilter::*;
    match (options.minification, options.magnification) {
        (Nearest, Nearest) => D3D11_FILTER_MIN_MAG_MIP_POINT,
        (Linear, Nearest) => D3D11_FILTER_MIN_LINEAR_MAG_MIP_POINT,
        (Nearest, Linear) => D3D11_FILTER_MIN_POINT_MAG_LINEAR_MIP_POINT,
        (Linear, Linear) => D3D11_FILTER_MIN_MAG_LINEAR_MIP_POINT
    }
}

fn make_box(x: usize, y: usize, w: usize, h: usize) -> D3D11_BOX {
    D3D11_BOX {
        left: x as u32,
        top: y as u32,
        front: 0,
        right: (x + w) as u32,
        bottom: (y + h) as u32,
        back: 1,
    }
}

fn make_resource<T>(func: impl FnOnce(Option<*mut Option<T>>) -> windows::core::Result<()>) -> T {
    unsafe {
        let mut obj = std::mem::MaybeUninit::zeroed();
        func(Some(obj.as_mut_ptr()))
            .expect("Resource creation failed");
        obj.assume_init()
            .expect("Returned resource is null")
    }
}

fn retrieve<S, T>(self_type: &S, func: unsafe fn(&S, *mut T)) -> T {
    unsafe {
        let mut desc = std::mem::MaybeUninit::zeroed();
        func(self_type, desc.as_mut_ptr());
        desc.assume_init()
    }
}