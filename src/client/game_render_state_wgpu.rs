use crate::client::GameFrame;
use image::GenericImageView;
use nalgebra_glm::*;
use std::path::Path;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TileVertex {
    tile_xyz: [f32; 3],
    tile_uv: [f32; 2],
    mask_uv: [f32; 2],
}

impl TileVertex {
    const VAO: &'static [wgpu::VertexAttribute] = 
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x2];

    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TileVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::VAO,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightVertex {
    light_xy: [f32; 2],
}

impl LightVertex {
    const VAO: &'static [wgpu::VertexAttribute] = 
        &wgpu::vertex_attr_array![0 => Float32x2];

    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LightVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::VAO,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_matrix: [[f32; 4]; 4],
}

impl CameraUniform {
    fn uniform_desc(&self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[self.clone()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });
        (camera_buffer, camera_bind_group_layout, camera_bind_group)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TileUniform {
    mul_rgb: [f32; 3],
}

impl TileUniform {
    fn uniform_desc(&self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let tile_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Tile Uniform Buffer"),
                contents: bytemuck::cast_slice(&[self.clone()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let tile_uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("tile_uniform_bind_group_layout"),
        });

        let tile_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &tile_uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tile_uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("tile_uniform_bind_group"),
        });

        (tile_uniform_buffer, tile_uniform_bind_group_layout, tile_uniform_bind_group)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    texture_size: [u32; 2],
}

impl LightUniform {
    fn uniform_desc(&self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let light_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light Uniform Buffer"),
                contents: bytemuck::cast_slice(&[self.clone()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let light_uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("light_uniform_bind_group_layout"),
        });

        let light_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("light_uniform_bind_group"),
        });
        (light_uniform_buffer, light_uniform_bind_group_layout, light_uniform_bind_group)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextureType {
    Rgba8,
    Gray,
    Rgba8Uint,
}

impl TextureType {
    fn format(self) -> wgpu::TextureFormat {
        match self {
            TextureType::Rgba8 => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureType::Gray => wgpu::TextureFormat::R8Unorm,
            TextureType::Rgba8Uint => wgpu::TextureFormat::Rgba8Uint,
        }
    }

    fn byte_size(self) -> u32 {
        match self {
            TextureType::Rgba8 => 4,
            TextureType::Gray => 1,
            TextureType::Rgba8Uint => 4,
        }
    }
}

fn create_texture(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8], tt: TextureType, label: Option<&'static str>) -> anyhow::Result<wgpu::Texture> {
    let image = image::load_from_memory(bytes)?;
    let (width, height) = image.dimensions();
    let data: Vec<u8>  = match tt {
        TextureType::Gray => image.into_luma8().into_vec(),
        TextureType::Rgba8 | TextureType::Rgba8Uint => image.into_rgba8().into_vec(),
    };
    let texture_size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(
        &wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: tt.format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label,
            view_formats: &[],
        }
    );
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * tt.byte_size()),
            rows_per_image: Some(height),
        },
        texture_size,
    );
    Ok(texture)
}

fn create_empty_texture(device: &wgpu::Device, (width, height): (u32, u32), tt: TextureType, label: Option<&'static str>) -> anyhow::Result<wgpu::Texture> {
    let texture_size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(
        &wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: tt.format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label,
            view_formats: &[],
        }
    );
    Ok(texture)
}

pub struct GameRenderStateWgpu {
    instance: wgpu::Instance,
    // pure unsafe hackery
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (i32, i32),

    tile_render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,

    fg_vertex_buffer: wgpu::Buffer,
    bg_vertex_buffer: wgpu::Buffer,
    light_vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    tile_uniform: TileUniform,
    tile_uniform_buffer: wgpu::Buffer,
    tile_uniform_bind_group: wgpu::BindGroup,

    tile_texture_bind_group: wgpu::BindGroup,
    
    light_uniform: LightUniform,
    light_uniform_buffer: wgpu::Buffer,
    light_uniform_bind_group: wgpu::BindGroup,

    light_texture_bind_group: wgpu::BindGroup,
    light_texture: wgpu::Texture,
}

impl GameRenderStateWgpu {
    pub async fn new(_root: &'static Path, window: &mut crate::window::Window) -> anyhow::Result<Self> {
        let size = window.window.get_size();
        let scale = window.window.get_content_scale();
        let size = ((size.0 as f32 * scale.0) as i32, (size.1 as f32 * scale.1) as i32);

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::empty(),
            gles_minor_version: Default::default(),
        });
        
        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // FIXME: This is an unsafe hack that leaks into safe code.
        let surface = unsafe { instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window.window)?) }?;

        let adapter =  instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await
        .ok_or_else(|| anyhow::anyhow!("WGPU: No adapter found"))?;

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ).await?;
 
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
            .copied()
            //.filter(|f| f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0 as u32,
            height: size.1 as u32,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let tile_sheet = create_texture(&device, &queue, include_bytes!("../../resources/tile_sheet.png"), TextureType::Rgba8, Some("tile_sheet"))?;
        let mask_sheet = create_texture(&device, &queue, include_bytes!("../../resources/mask_sheet.png"), TextureType::Gray, Some("mask_sheet"))?;
        let light_texture = create_empty_texture(&device, (256, 256), TextureType::Rgba8Uint, Some("light_texture"))?;

        let tile_sheet_texture_view = tile_sheet.create_view(&wgpu::TextureViewDescriptor::default());
        let mask_sheet_texture_view = mask_sheet.create_view(&wgpu::TextureViewDescriptor::default());
        let light_texture_view = light_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let tile_texture_bind_group = device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&tile_sheet_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&mask_sheet_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&texture_sampler),
                        }
                    ],
                    label: Some("tile_sheet_bind_group"),
                }
            );

        let light_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Uint,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("light_texture_bind_group_layout"),
            });

        let light_texture_bind_group = device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &light_texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&light_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&texture_sampler),
                        }
                    ],
                    label: Some("light_texture_bind_group"),
                }
            );
            
        let tile_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/tile.wgsl"));
        let light_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/light.wgsl"));

        let camera_uniform = CameraUniform {
            view_matrix: Mat4::identity().into(),
        };
        let (camera_buffer, camera_bind_group_layout, camera_bind_group) = camera_uniform.uniform_desc(&device);

        let tile_uniform = TileUniform {
            mul_rgb: [1.0, 1.0, 1.0],
        };
        let (tile_uniform_buffer, tile_uniform_bind_group_layout, tile_uniform_bind_group) = tile_uniform.uniform_desc(&device);

        let tile_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &texture_bind_group_layout,
                    &tile_uniform_bind_group_layout,
                ],
                push_constant_ranges: &[],
        });

        let tile_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&tile_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &tile_shader,
                entry_point: "vs_main",
                buffers: &[
                    TileVertex::desc(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &tile_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,//Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let light_uniform = LightUniform {
            texture_size: [256, 256],
        };
        let (light_uniform_buffer, light_uniform_bind_group_layout, light_uniform_bind_group) = light_uniform.uniform_desc(&device);

        let light_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &light_texture_bind_group_layout,
                    &light_uniform_bind_group_layout,
                ],
                push_constant_ranges: &[],
        });

        let light_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Light Render Pipeline"),
            layout: Some(&light_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &light_shader,
                entry_point: "vs_main",
                buffers: &[
                    LightVertex::desc(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &light_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Dst,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Dst,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,//Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_data: Vec<TileVertex> = vec![];
        let fg_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("FG Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let bg_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("BG Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let light_vertex_data: Vec<LightVertex> = vec![];
        let light_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light Vertex Buffer"),
                contents: bytemuck::cast_slice(&light_vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        #[rustfmt::skip]
        let ibo_data: Vec<u16> = (0..13107)
            .into_iter()
            .flat_map(|i| [i * 4 + 0, i * 4 + 3, i * 4 + 1, i * 4 + 2, u16::MAX])
            .collect();
        assert_eq!(ibo_data.len(), 65535);
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&ibo_data),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        let num_indices = ibo_data.len() as u32;


        Ok(
            Self {
                instance,
                surface,
                device,
                queue,
                config,
                size,
                tile_render_pipeline,
                light_render_pipeline,
                fg_vertex_buffer,
                bg_vertex_buffer,
                light_vertex_buffer,
                index_buffer,
                num_indices,
                camera_uniform,
                camera_buffer,
                camera_bind_group,
                tile_uniform,
                tile_uniform_buffer,
                tile_uniform_bind_group,
                tile_texture_bind_group,
                light_uniform,
                light_uniform_buffer,
                light_uniform_bind_group,
                light_texture_bind_group,
                light_texture,
            }
        )
    }

    pub fn resize(&mut self, new_size: (i32, i32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            self.size = new_size;
            self.config.width = new_size.0 as u32;
            self.config.height = new_size.1 as u32;
            self.surface.configure(&self.device, &self.config);
        }   
    }

    /// # Safety
    /// - It's not lmao
    pub fn update_surface(&mut self, window: &mut crate::window::Window) {
        unsafe { self.instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window.window).unwrap()) }.unwrap();
    }

    fn update_camera(&mut self, game_frame: &GameFrame) {
        let view = {
            let view = Mat3::identity();
            let view = view
                * scaling2d(&Vec2::new(
                     2. / game_frame.viewport_w,
                     -2. / game_frame.viewport_h,
                ));
            let view = view
                * translation2d(&Vec2::new(
                    -game_frame.viewport_x - game_frame.viewport_w / 2.,
                    -game_frame.viewport_y - game_frame.viewport_h / 2.,
                ));
            nalgebra_glm::mat3_to_mat4(&view)
        };
        self.camera_uniform.view_matrix = view.into();
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    fn get_tile_vertices(&self, game_frame: &GameFrame) -> (Vec<TileVertex>, Vec<TileVertex>) {
        // Render tiles.
        let max_tiles = (game_frame.tiles_w - 2) * (game_frame.tiles_h - 2);
        let mut fg_vertex_tiles = Vec::with_capacity(max_tiles);
        let mut bg_vertex_tiles = Vec::with_capacity(max_tiles);
        'skip: {
            if max_tiles == 0 {
                break 'skip;
            }

            // Calculate tile data and upload to GPU.
            'calc_tiles: {
                for y in 1..game_frame.tiles_h - 1 {
                    'x: for x in 1..game_frame.tiles_w - 1 {
                        let index = x + y * game_frame.tiles_w;

                        // Fill FG.
                        'skip_fg: {
                            let tile = game_frame.fg_tiles[index];
                            if tile == 0 {
                                break 'skip_fg;
                            }

                            // Fill vertex data.
                            let tx = 16. * (x + game_frame.tiles_x) as f32;
                            let ty = 16. * (y + game_frame.tiles_y) as f32;

                            let uv_tx = 16. * tile as f32;

                            let tw = game_frame.tiles_w;
                            let t0 = ((tile > game_frame.fg_tiles[index - tw]) as u8) << 0;
                            let t1 = ((tile > game_frame.fg_tiles[index - tw + 1]) as u8) << 1;
                            let t2 = ((tile > game_frame.fg_tiles[index + 1]) as u8) << 2;
                            let t3 = ((tile > game_frame.fg_tiles[index + tw + 1]) as u8) << 3;
                            let u = ((t0 | t1 | t2 | t3) << 2) as f32;
                            let t4 = ((tile > game_frame.fg_tiles[index + tw]) as u8) << 0;
                            let t5 = ((tile > game_frame.fg_tiles[index + tw - 1]) as u8) << 1;
                            let t6 = ((tile > game_frame.fg_tiles[index - 1]) as u8) << 2;
                            let t7 = ((tile > game_frame.fg_tiles[index - tw - 1]) as u8) << 3;
                            let v = ((t4 | t5 | t6 | t7) << 2) as f32;
                            #[rustfmt::skip]
                            fg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx - 8.,       ty - 8.,       tile as f32],
                                tile_uv:  [uv_tx, 0.],
                                mask_uv:  [u, v],
                            });
                            fg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx + 16. + 8., ty - 8.,       tile as f32],
                                tile_uv:  [16. + uv_tx, 0.],
                                mask_uv:  [u + 4., v],
                            });
                            fg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx + 16. + 8., ty + 16. + 8., tile as f32],
                                tile_uv:  [16. + uv_tx, 16.],
                                mask_uv:  [u + 4., v + 4.],
                            });
                            fg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx - 8.,       ty + 16. + 8., tile as f32],
                                tile_uv:  [0. + uv_tx,  16.],
                                mask_uv:  [u,      v + 4.],
                            });

                            // Skip check bg tile.
                            continue 'x;
                        }

                        // Fill BG.
                        'skip_bg: {
                            let tile = game_frame.bg_tiles[index];
                            if tile == 0 {
                                break 'skip_bg;
                            }

                            // Fill vertex data.
                            let tx = 16. * (x + game_frame.tiles_x) as f32;
                            let ty = 16. * (y + game_frame.tiles_y) as f32;

                            let uv_tx = 16. * tile as f32;
                            let tw = game_frame.tiles_w;
                            let t0 = ((tile > game_frame.bg_tiles[index - tw]) as u8) << 0;
                            let t1 = ((tile > game_frame.bg_tiles[index - tw + 1]) as u8) << 1;
                            let t2 = ((tile > game_frame.bg_tiles[index + 1]) as u8) << 2;
                            let t3 = ((tile > game_frame.bg_tiles[index + tw + 1]) as u8) << 3;
                            let u = ((t0 | t1 | t2 | t3) << 2) as f32;
                            let t4 = ((tile > game_frame.bg_tiles[index + tw]) as u8) << 0;
                            let t5 = ((tile > game_frame.bg_tiles[index + tw - 1]) as u8) << 1;
                            let t6 = ((tile > game_frame.bg_tiles[index - 1]) as u8) << 2;
                            let t7 = ((tile > game_frame.bg_tiles[index - tw - 1]) as u8) << 3;
                            let v = ((t4 | t5 | t6 | t7) << 2) as f32;
                            bg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx - 8.,       ty - 8.,       tile as f32],
                                tile_uv:  [uv_tx, 0.],
                                mask_uv:  [u, v],
                            });
                            bg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx + 16. + 8., ty - 8.,       tile as f32],
                                tile_uv:  [16. + uv_tx, 0.],
                                mask_uv:  [u + 4., v],
                            });
                            bg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx + 16. + 8., ty + 16. + 8., tile as f32],
                                tile_uv:  [16. + uv_tx, 16.],
                                mask_uv:  [u + 4., v + 4.],
                            });
                            bg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx - 8.,       ty + 16. + 8., tile as f32],
                                tile_uv:  [0. + uv_tx,  16.],
                                mask_uv:  [u,      v + 4.],
                            });
                        }
                    }
                }

                break 'calc_tiles
            };
        }
        (fg_vertex_tiles, bg_vertex_tiles)
    }

    fn update_tiles(&mut self, game_frame: &GameFrame) -> (Vec<TileVertex>, Vec<TileVertex>) {
        let (fg_vertex_tiles, bg_vertex_tiles) = self.get_tile_vertices(game_frame);

        // TODO: test if copying is faster than updating if we care
        self.fg_vertex_buffer.destroy();
        self.fg_vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("FG Vertex Buffer"),
                contents: bytemuck::cast_slice(&fg_vertex_tiles),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        self.bg_vertex_buffer.destroy();
        self.bg_vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("BG Vertex Buffer"),
                contents: bytemuck::cast_slice(&bg_vertex_tiles),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        (fg_vertex_tiles, bg_vertex_tiles)
    }

    fn update_lighting(&mut self, game_frame: &GameFrame) -> Vec<LightVertex> {
        // Render lighting.
        let mut light_vertices = vec![];
        self.light_uniform.texture_size = [game_frame.light_w as u32, game_frame.light_h as u32];
        self.queue.write_buffer(&self.light_uniform_buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));
        {
            // Set up.
            let mut rgba = vec![0u8; game_frame.light_w  * game_frame.light_h * 4];
            for i in 0 .. game_frame.light_w  * game_frame.light_h  {
                rgba[4 * i + 0] = game_frame.r_channel[i];
                rgba[4 * i + 1] = game_frame.g_channel[i];
                rgba[4 * i + 2] = game_frame.b_channel[i];
                rgba[4 * i + 3] = 255;
            }
            light_vertices.push(
                LightVertex {
                    light_xy: [game_frame.light_x as f32 * 16., game_frame.light_y as f32 * 16.],
                },
            );
            light_vertices.push(
                LightVertex {
                    light_xy: [(game_frame.light_x + game_frame.light_w) as f32 * 16., game_frame.light_y as f32 * 16.],
                },
            );
            light_vertices.push(
                LightVertex {
                    light_xy: [(game_frame.light_x + game_frame.light_w) as f32 * 16., (game_frame.light_y + game_frame.light_h) as f32 * 16.],
                },
            );
            light_vertices.push(
                LightVertex {
                    light_xy: [game_frame.light_x as f32 * 16., (game_frame.light_y + game_frame.light_h) as f32 * 16.],
                },
            );
            // TODO:
            let texture_size = wgpu::Extent3d {
                height: game_frame.light_h as u32,
                width: game_frame.light_w as u32,
                depth_or_array_layers: 1,
            };
            self.queue.write_texture(
                // Tells wgpu where to copy the pixel data
                wgpu::ImageCopyTexture {
                    texture: &self.light_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                // The actual pixel data
                &rgba,
                // The layout of the texture
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * game_frame.light_w as u32),
                    rows_per_image: Some(game_frame.light_h as u32),
                },
                texture_size,
            );
        }
        self.light_vertex_buffer.destroy();
        self.light_vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light Vertex Buffer"),
                contents: bytemuck::cast_slice(&light_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        light_vertices
    }

    pub fn render(&mut self, _ts: u64, game_frame: GameFrame) {
        self.update_camera(&game_frame);
        let (fg_vertex_tiles, bg_vertex_tiles) = self.update_tiles(&game_frame);
        let light_vertices = self.update_lighting(&game_frame);

        let output = self.surface.get_current_texture().unwrap();//?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                // This is what @location(0) in the fragment shader targets
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0x15 as f64 / 256.,
                            g: 0x9F as f64 / 256.,
                            b: 0xEA as f64 / 256.,
                            a: 1.0,
                        }),
                        // TODO:
                        store: Default::default(),
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });


            // background tiles
            render_pass.set_pipeline(&self.tile_render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.tile_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.tile_uniform_bind_group, &[]);

            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
            assert!(self.num_indices >= fg_vertex_tiles.len() as u32, "Ran out of indices in the IBO!");
            assert!(self.num_indices >= bg_vertex_tiles.len() as u32, "Ran out of indices in the IBO!");

            // Update uniform for BG tile color
            self.tile_uniform.mul_rgb = [0.7, 0.7, 0.8];
            self.queue.write_buffer(&self.tile_uniform_buffer, 0, bytemuck::cast_slice(&[self.tile_uniform]));
            render_pass.set_vertex_buffer(0, self.bg_vertex_buffer.slice(..));

            let num_bg_tiles = (bg_vertex_tiles.len() / 4) * 5;
            let idx_val = std::cmp::min(self.num_indices, num_bg_tiles as u32);
            render_pass.draw_indexed(0..idx_val, 0, 0..1);

            // foreground tiles
            // We reuse the setup from above but change the uniform
            self.tile_uniform.mul_rgb = [1.0, 1.0, 1.0];
            self.queue.write_buffer(&self.tile_uniform_buffer, 0, bytemuck::cast_slice(&[self.tile_uniform]));
            render_pass.set_vertex_buffer(0, self.fg_vertex_buffer.slice(..));

            let num_fg_tiles = (fg_vertex_tiles.len() / 4) * 5;
            let idx_val = std::cmp::min(self.num_indices, num_fg_tiles as u32);
            render_pass.draw_indexed(0..idx_val, 0, 0..1);

            // Light!
            // Reuse the index buffer
            render_pass.set_pipeline(&self.light_render_pipeline);
            //render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_vertex_buffer(0, self.light_vertex_buffer.slice(..));
            render_pass.set_bind_group(1, &self.light_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.light_uniform_bind_group, &[]);
            let idx_val = std::cmp::min(self.num_indices, light_vertices.len() as u32);
            render_pass.draw_indexed(0..idx_val, 0, 0..1);
        }


        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
