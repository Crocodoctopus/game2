pub const MAX_LIGHTMAP_SIZE: usize = 512;

use crate::client::log;
use crate::shared::item::ItemKind;
use crate::shared::tile::{TILE_SIZE, TILE_TEXTURE_PROPERTIES, TileKind, TileTextureProperty};
use crate::window::*;
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext, Version,
};
use glutin::display::GetGlDisplay;
use glutin::prelude::{GlDisplay, NotCurrentGlContext};
use glutin::surface::{GlSurface, Surface, WindowSurface};
use glutin_winit::GlWindow;
use nalgebra_glm::*;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::offset_of;
use std::path::Path;
use winit::raw_window_handle::HasWindowHandle;

use super::game_render_desc::{GameRenderDesc, HumanoidRenderDesc, ItemRenderDesc};

#[derive(Copy, Clone, Debug, Default)]
struct TileVertex {
    xyz: GlVec3,
    uv: GlVec2,
    mask_uv: GlVec2,
}

#[derive(Copy, Clone, Debug, Default)]
struct SpriteVertex {
    xy: GlVec3,
    uv: GlVec2,
}

#[allow(dead_code)]
pub struct GameRenderState {
    context: PossiblyCurrentContext,
    surface: Surface<WindowSurface>,

    // General purpose.
    global_vao: GlHandle,
    quad_ibo: GlHandle, // u16

    // Generic texture map.
    textures: HashMap<&'static str, GlHandle>,

    //
    //sprite_texture: GlHandle,
    sprite_vertices: GlHandle,
    sprite_program: GlHandle,

    // Light rendering.
    light_texture: GlHandle,
    light_program: GlHandle,

    // Tile rendering.
    tile_mask_texture: GlHandle, // R
    tile_program: GlHandle,
    fg_tile_vertices: GlHandle,
    bg_tile_vertices: GlHandle,
}

#[derive(Debug)]
struct GlHandle(pub gl::types::GLuint);

impl GlHandle {
    fn null() -> Self {
        Self(0)
    }

    fn is_null(&self) -> bool {
        self.0 == 0
    }
}

#[allow(unused)]
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
struct GlVec2(f32, f32);

#[allow(unused)]
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
struct GlVec3(f32, f32, f32);

impl GameRenderState {
    pub fn new(
        root: &'static Path,
        context: NotCurrentContext,
        surface: Surface<WindowSurface>,
    ) -> Self {
        let context = context.make_current(&surface).unwrap();

        let check_shader_error = |shader: u32| unsafe {
            let mut success: i32 = 2;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            assert_ne!(success, 2);
            if success == gl::FALSE as i32 {
                let mut size: i32 = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut size);
                let mut bytes = vec![0u8; size as usize];
                gl::GetShaderInfoLog(shader, size, &mut size, bytes.as_mut_ptr() as *mut i8);
                return Err(String::from_utf8_lossy(&bytes[0..size as usize]).into_owned());
            }
            Ok(())
        };

        let check_program_error = |program: u32| unsafe {
            let mut success: i32 = 2;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
            assert_ne!(success, 2);
            if success == gl::FALSE as i32 {
                let mut size: i32 = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut size);
                let mut bytes = vec![0u8; size as usize];
                gl::GetProgramInfoLog(program, size, &mut size, bytes.as_mut_ptr() as *mut i8);
                return Err(String::from_utf8_lossy(&bytes[0..size as usize]).into_owned());
            }
            Ok(())
        };

        let gen_buffer = || unsafe {
            let mut handle = GlHandle::null();
            gl::GenBuffers(1, &mut handle.0);
            (handle.0 > 0).then_some(handle)
        };

        let create_shader = |src: &[u8], kind: gl::types::GLenum| unsafe {
            let src_ptr = src.as_ptr() as *const i8;
            let src_len = src.len() as i32;
            let shader = gl::CreateShader(kind);
            assert_ne!(shader, 0);
            gl::ShaderSource(shader, 1, &src_ptr, &src_len);
            gl::CompileShader(shader);
            check_shader_error(shader).map(|_| GlHandle(shader))
        };

        let create_program = |vert_shader: GlHandle, frag_shader: GlHandle| unsafe {
            let program = gl::CreateProgram();
            gl::AttachShader(program, vert_shader.0);
            gl::AttachShader(program, frag_shader.0);
            gl::LinkProgram(program);
            gl::DeleteShader(vert_shader.0);
            gl::DeleteShader(frag_shader.0);
            check_program_error(program).map(|_| GlHandle(program))
        };

        unsafe {
            use gl::types::*;
            extern "system" fn gl_debug_callback(
                _source: GLenum,
                _ptype: GLenum,
                _id: GLuint,
                severity: GLenum,
                _length: GLsizei,
                message: *const GLchar,
                _user_param: *mut c_void,
            ) {
                // Filter out info notifications.
                if severity != gl::DEBUG_SEVERITY_NOTIFICATION {
                    let msg = unsafe { std::ffi::CStr::from_ptr(message) };
                    log!("GL CALLBACK: {msg:?}");
                }
            }

            gl::Enable(gl::DEBUG_OUTPUT);
            gl::Enable(gl::BLEND);
            gl::Enable(gl::PRIMITIVE_RESTART);
            gl::Enable(gl::TEXTURE_2D);
            gl::DebugMessageCallback(Some(gl_debug_callback), std::ptr::null());

            //
            let mut textures = HashMap::new();

            // Load tile texture into gpu.
            {
                let path = "tile_sheet.png";
                let data = std::fs::read(root.join("resources").join(path)).unwrap();
                let texture_data = image::load_from_memory(&data[..]).unwrap();
                let mut texture_handle = GlHandle::null();
                gl::GenTextures(1, &mut texture_handle.0);
                assert!(!texture_handle.is_null());
                gl::BindTexture(gl::TEXTURE_2D, texture_handle.0);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGB as i32,
                    texture_data.width() as i32,
                    texture_data.height() as i32,
                    0,
                    gl::RGB,
                    gl::UNSIGNED_BYTE,
                    texture_data.into_rgb8().as_ptr() as *const c_void,
                );
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                textures.insert(path, texture_handle);
            }

            // Load mask texture into gpu.
            let tile_mask_texture = {
                let path = "mask_sheet.png";
                let data = std::fs::read(root.join("resources").join(path)).unwrap();
                let texture_data = image::load_from_memory(&data[..]).unwrap();
                let mut texture_handle = GlHandle::null();
                gl::GenTextures(1, &mut texture_handle.0);
                gl::BindTexture(gl::TEXTURE_2D, texture_handle.0);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::R8I as i32,
                    texture_data.width() as i32,
                    texture_data.height() as i32,
                    0,
                    gl::RED_INTEGER,
                    gl::UNSIGNED_BYTE,
                    texture_data.into_luma8().as_ptr() as *const c_void,
                );
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                texture_handle
            };

            // Create light texture.
            let light_texture = {
                let mut texture_handle = GlHandle::null();
                gl::GenTextures(1, &mut texture_handle.0);
                gl::BindTexture(gl::TEXTURE_2D, texture_handle.0);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA8UI as i32,
                    512,
                    512,
                    0,
                    gl::RGBA_INTEGER,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null(),
                );
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                texture_handle
            };

            // Bind the VAO and never touch it again.
            let mut global_vao = GlHandle::null();
            gl::GenVertexArrays(1, &mut global_vao.0);
            assert!(!global_vao.is_null());
            gl::BindVertexArray(global_vao.0);

            let quad_ibo = gen_buffer().unwrap();
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, quad_ibo.0);
            let ibo_data: Box<[u16]> = (0..u16::MAX / 5)
                .flat_map(|i| [4 * i, 4 * i + 3, 4 * i + 1, 4 * i + 2, u16::MAX])
                .collect();
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                ibo_data.len() as isize,
                ibo_data.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            let fg_tile_vertices = gen_buffer().unwrap();
            let bg_tile_vertices = gen_buffer().unwrap();

            let sprite_vertices = gen_buffer().unwrap();

            let tile_program = create_program(
                create_shader(
                    include_bytes!("./shaders/tile.glsl.vert"),
                    gl::VERTEX_SHADER,
                )
                .unwrap(),
                create_shader(
                    include_bytes!("./shaders/tile.glsl.frag"),
                    gl::FRAGMENT_SHADER,
                )
                .unwrap(),
            )
            .unwrap();

            let light_program = create_program(
                create_shader(
                    include_bytes!("./shaders/light.glsl.vert"),
                    gl::VERTEX_SHADER,
                )
                .unwrap(),
                create_shader(
                    include_bytes!("./shaders/light.glsl.frag"),
                    gl::FRAGMENT_SHADER,
                )
                .unwrap(),
            )
            .unwrap();

            let sprite_program = create_program(
                create_shader(
                    include_bytes!("./shaders/sprite.glsl.vert"),
                    gl::VERTEX_SHADER,
                )
                .unwrap(),
                create_shader(
                    include_bytes!("./shaders/sprite.glsl.frag"),
                    gl::FRAGMENT_SHADER,
                )
                .unwrap(),
            )
            .unwrap();

            Self {
                context,
                surface,

                global_vao,
                quad_ibo,

                textures,

                //sprite_texture,
                sprite_program,
                sprite_vertices,

                light_texture,
                light_program,

                tile_mask_texture,
                tile_program,
                fg_tile_vertices,
                bg_tile_vertices,
            }
        }
    }

    pub fn handle_events(&mut self, input_events: impl Iterator<Item = InputEvent>) -> bool {
        for event in input_events {
            match event {
                InputEvent::WindowClose => return true,

                InputEvent::WindowResize { width, height } => {}

                // Most events are ignored.
                _ => {}
            }
        }

        false
    }

    pub fn render(&mut self, _ts: u64, game_render_desc: &GameRenderDesc) {
        // Compute view matrix.
        let view_matrix = {
            let view = Mat3::identity();
            let view = view
                * scaling2d(&Vec2::new(
                    2. / game_render_desc.viewport_w,
                    -2. / game_render_desc.viewport_h,
                ));
            view * translation2d(&Vec2::new(
                -game_render_desc.viewport_x - game_render_desc.viewport_w / 2.,
                -game_render_desc.viewport_y - game_render_desc.viewport_h / 2.,
            ))
        };

        // Upload light data to gpu.
        {
            let w = game_render_desc.light_w;
            let h = game_render_desc.light_h;
            let mut data = vec![0u8; 4 * w * h];
            for i in 0..w * h {
                data[4 * i] = game_render_desc.r_channel[i].raw();
                data[4 * i + 1] = game_render_desc.g_channel[i].raw();
                data[4 * i + 2] = game_render_desc.b_channel[i].raw();
            }
            assert!(w < 512);
            assert!(h < 512);
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, self.light_texture.0);
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    game_render_desc.light_w as i32,
                    game_render_desc.light_h as i32,
                    gl::RGBA_INTEGER,
                    gl::UNSIGNED_BYTE,
                    data.as_ptr() as *const c_void,
                );
            }
        }

        // Generate sprite vertex data.

        // Generate sprites for Humanoids.
        let mut sprite_vertices = Vec::with_capacity(game_render_desc.humanoids.len());
        for &HumanoidRenderDesc { x, y, u, v, w, h } in &game_render_desc.humanoids {
            sprite_vertices.extend_from_slice(&[
                SpriteVertex {
                    xy: GlVec3(x, y, 0.),
                    uv: GlVec2(u, v),
                },
                SpriteVertex {
                    xy: GlVec3(x + w, y, 0.),
                    uv: GlVec2(u + w, v),
                },
                SpriteVertex {
                    xy: GlVec3(x + w, y + h, 0.),
                    uv: GlVec2(u + w, v + h),
                },
                SpriteVertex {
                    xy: GlVec3(x, y + h, 0.),
                    uv: GlVec2(u, v + h),
                },
            ]);
        }
        let sprite_count = sprite_vertices.len() / 4;

        // Upload sprite data to GPU.
        if sprite_count > 0 {
            unsafe {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.sprite_vertices.0);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (sprite_vertices.len() * size_of::<SpriteVertex>()) as isize,
                    sprite_vertices.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );
            }
        }

        // Generate sprites for Items.
        /*for &ItemRenderDesc { x, y, kind } in &game_render_desc.items {
            let (w, h, u, v) = match kind {
                ItemKind::Tile(_tile) => (TILE_SIZE as f32, TILE_SIZE as f32, 0., 0.),
                #[allow(unreachable_patterns)]
                _ => unimplemented!(),
            };

            sprite_data
                .entry("default")
                .or_default()
                .extend_from_slice(&[
                    SpriteVertex {
                        xy: GlVec3(x, y, 0.),
                        uv: GlVec2(u, v),
                    },
                    SpriteVertex {
                        xy: GlVec3(x + w, y, 0.),
                        uv: GlVec2(u + w, v),
                    },
                    SpriteVertex {
                        xy: GlVec3(x + w, y + h, 0.),
                        uv: GlVec2(u + w, v + h),
                    },
                    SpriteVertex {
                        xy: GlVec3(x, y + h, 0.),
                        uv: GlVec2(u, v + h),
                    },
                ]);
        }*/

        // Generate tile vertex data from game render descriptor.
        let (fg_tile_vertices, bg_tile_vertices) = generate_tile_data(game_render_desc);
        let bg_tile_count = bg_tile_vertices.len() / 4;
        let fg_tile_count = fg_tile_vertices.len() / 4;

        // Upload background tile vertex data to GPU.
        if bg_tile_count > 0 {
            unsafe {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.bg_tile_vertices.0);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (bg_tile_vertices.len() * size_of::<TileVertex>()) as isize,
                    bg_tile_vertices.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );
            }
        }

        // Upload foreground tile vertex data to GPU.
        if fg_tile_count > 0 {
            unsafe {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.fg_tile_vertices.0);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (fg_tile_vertices.len() * size_of::<TileVertex>()) as isize,
                    fg_tile_vertices.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );
            }
        }

        // Clear.
        unsafe {
            gl::ClearColor(0., 0., 0., 1.);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        // Rendering background tiles.
        if bg_tile_count > 0 {
            unsafe {
                gl::UseProgram(self.tile_program.0);

                gl::EnableVertexAttribArray(0);
                gl::VertexAttribFormat(
                    0,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    offset_of!(TileVertex, xyz) as u32,
                );
                gl::BindVertexBuffer(
                    0,
                    self.bg_tile_vertices.0,
                    0,
                    size_of::<TileVertex>() as i32,
                );

                gl::EnableVertexAttribArray(1);
                gl::VertexAttribFormat(
                    1,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    offset_of!(TileVertex, uv) as u32,
                );
                gl::BindVertexBuffer(
                    1,
                    self.bg_tile_vertices.0,
                    0,
                    size_of::<TileVertex>() as i32,
                );

                gl::EnableVertexAttribArray(2);
                gl::VertexAttribFormat(
                    2,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    offset_of!(TileVertex, mask_uv) as u32,
                );
                gl::BindVertexBuffer(
                    2,
                    self.bg_tile_vertices.0,
                    0,
                    size_of::<TileVertex>() as i32,
                );

                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, self.textures["tile_sheet.png"].0);
                gl::ActiveTexture(gl::TEXTURE1);
                gl::BindTexture(gl::TEXTURE_2D, self.tile_mask_texture.0);

                gl::UniformMatrix3fv(0, 1, gl::FALSE, view_matrix.as_ptr());
                gl::Uniform1i(1, 0);
                gl::Uniform1i(2, 1);
                gl::Uniform4f(3, 0.6, 0.6, 0.6, 1.0);

                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.quad_ibo.0);
                gl::PrimitiveRestartIndex(u16::MAX as u32);

                gl::DrawElements(
                    gl::TRIANGLE_STRIP,
                    bg_tile_count as i32 * 5,
                    gl::UNSIGNED_SHORT,
                    std::ptr::null(),
                );
            }
        }

        // Render sprites.
        if sprite_count > 0 {
            unsafe {
                gl::UseProgram(self.sprite_program.0);

                gl::EnableVertexAttribArray(0);
                gl::VertexAttribFormat(
                    0,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    offset_of!(SpriteVertex, xy) as u32,
                );
                gl::BindVertexBuffer(
                    0,
                    self.sprite_vertices.0,
                    0,
                    size_of::<SpriteVertex>() as i32,
                );

                gl::EnableVertexAttribArray(1);
                gl::VertexAttribFormat(
                    1,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    offset_of!(SpriteVertex, uv) as u32,
                );
                gl::BindVertexBuffer(
                    0,
                    self.sprite_vertices.0,
                    0,
                    size_of::<SpriteVertex>() as i32,
                );

                // Attach uniforms.
                let model_matrix = Mat3::identity();
                gl::UniformMatrix3fv(0, 1, gl::FALSE, view_matrix.as_ptr());
                gl::UniformMatrix3fv(1, 1, gl::FALSE, model_matrix.as_ptr());

                // Draw.
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.quad_ibo.0);
                gl::PrimitiveRestartIndex(u16::MAX as u32);
                gl::DrawElements(
                    gl::TRIANGLE_STRIP,
                    sprite_count as i32 * 5,
                    gl::UNSIGNED_SHORT,
                    std::ptr::null(),
                );
            }
        }

        // Render foreground tiles.
        if fg_tile_count > 0 {
            unsafe {
                gl::UseProgram(self.tile_program.0);

                gl::EnableVertexAttribArray(0);
                gl::VertexAttribFormat(
                    0,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    offset_of!(TileVertex, xyz) as u32,
                );
                gl::BindVertexBuffer(
                    0,
                    self.fg_tile_vertices.0,
                    0,
                    size_of::<TileVertex>() as i32,
                );

                gl::EnableVertexAttribArray(1);
                gl::VertexAttribFormat(
                    1,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    offset_of!(TileVertex, uv) as u32,
                );
                gl::BindVertexBuffer(
                    1,
                    self.fg_tile_vertices.0,
                    0,
                    size_of::<TileVertex>() as i32,
                );

                gl::EnableVertexAttribArray(2);
                gl::VertexAttribFormat(
                    2,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    offset_of!(TileVertex, mask_uv) as u32,
                );
                gl::BindVertexBuffer(
                    2,
                    self.fg_tile_vertices.0,
                    0,
                    size_of::<TileVertex>() as i32,
                );

                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, self.textures["tile_sheet.png"].0);
                gl::ActiveTexture(gl::TEXTURE1);
                gl::BindTexture(gl::TEXTURE_2D, self.tile_mask_texture.0);

                gl::UniformMatrix3fv(0, 1, gl::FALSE, view_matrix.as_ptr());
                gl::Uniform1i(1, 0);
                gl::Uniform1i(2, 1);
                gl::Uniform4f(3, 1., 1., 1., 1.);

                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.quad_ibo.0);
                gl::PrimitiveRestartIndex(u16::MAX as u32);

                gl::DrawElements(
                    gl::TRIANGLE_STRIP,
                    fg_tile_count as i32 * 5,
                    gl::UNSIGNED_SHORT,
                    std::ptr::null(),
                );
            }
        }

        // Render light map.
        unsafe {
            // Use program.
            gl::UseProgram(self.light_program.0);

            // Map textures to texture units.
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.light_texture.0);

            // Attach uniforms.
            gl::UniformMatrix3fv(0, 1, gl::FALSE, view_matrix.as_ptr());
            gl::Uniform4f(
                1,
                game_render_desc.light_x as f32 * 16.,
                game_render_desc.light_y as f32 * 16.,
                game_render_desc.light_w as f32 * 16.,
                game_render_desc.light_h as f32 * 16.,
            );
            gl::Uniform2f(
                2,
                game_render_desc.light_w as f32,
                game_render_desc.light_h as f32,
            );
            gl::Uniform1i(3, 0);

            // Draw.
            gl::BlendFunc(gl::DST_COLOR, gl::ZERO);
            gl::BlendEquation(gl::FUNC_ADD);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.quad_ibo.0);
            gl::DrawElements(gl::TRIANGLE_STRIP, 4, gl::UNSIGNED_SHORT, std::ptr::null());
        }

        self.surface.swap_buffers(&self.context).unwrap();
    }
}

pub fn gl_create_context(window: &Window) -> (NotCurrentContext, Surface<WindowSurface>) {
    // Create context from window.
    let rwh = window.window.window_handle().ok().map(|wh| wh.as_raw());
    let context_attributes = ContextAttributesBuilder::default()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 3))))
        .build(rwh);
    let gl_display = window.gl_config.display();
    let context = unsafe {
        gl_display
            .create_context(&window.gl_config, &context_attributes)
            .unwrap()
    };

    // Surface?
    let surface_attributes = window
        .window
        .build_surface_attributes(Default::default())
        .unwrap();
    let surface = unsafe {
        window
            .gl_config
            .display()
            .create_window_surface(&window.gl_config, &surface_attributes)
            .unwrap()
    };

    gl::load_with(|s| {
        let s = std::ffi::CString::new(s).unwrap();
        surface.display().get_proc_address(s.as_c_str()).cast()
    });

    (context, surface)
}

fn generate_tile_data(game_render_desc: &GameRenderDesc) -> (Vec<TileVertex>, Vec<TileVertex>) {
    let max_tiles = (game_render_desc.tiles_w - 2) * (game_render_desc.tiles_h - 2);
    let mut fg_tile_vertices: Vec<TileVertex> = Vec::with_capacity(4 * max_tiles);
    let mut bg_tile_vertices: Vec<TileVertex> = Vec::with_capacity(4 * max_tiles);

    // Calculate tile data and upload to GPU.
    let tile_texture_properties_lookup = &crate::shared::tile::TILE_TEXTURE_PROPERTIES;
    let stride = game_render_desc.tiles_w;
    for y in 1..game_render_desc.tiles_h - 1 {
        for x in 1..game_render_desc.tiles_w - 1 {
            let tile_size_f32 = TILE_SIZE as f32;
            let index = x + y * game_render_desc.tiles_w;

            // Skip FG if tile is None.
            if !matches!(game_render_desc.fg_tiles[index].0, TileKind::None) {
                // Get tile properties.
                let tile_texture_properties =
                    tile_texture_properties_lookup[game_render_desc.fg_tiles[index].0 as usize];

                // Get texture UV.
                let u = tile_texture_properties.u;
                let v = tile_texture_properties.v;

                // Get depth.
                let depth = tile_texture_properties.depth;

                // Calculate position.
                let x = tile_size_f32 * (x + game_render_desc.tiles_x) as f32;
                let y = tile_size_f32 * (y + game_render_desc.tiles_y) as f32;
                let z = depth as f32;

                // Calculate mask UV.
                #[rustfmt::skip]
                        let mask_u = [ index - stride, index - stride + 1, index + 1, index + stride + 1 ]
                            .into_iter()
                            .rev()
                            .map(|index| game_render_desc.fg_tiles[index].0)
                            .map(|tile| tile_texture_properties_lookup[tile as usize].depth)
                            .map(|dep| (depth > dep) as u8)
                            .reduce(|acc, bit| (acc << 1) | bit)
                            .map(|out| (out << 2) as f32)
                            .unwrap();
                #[rustfmt::skip]
                        let mask_v  = [index + stride, index + stride - 1, index - 1, index - stride - 1 ]
                            .into_iter()
                            .rev()
                            .map(|index| game_render_desc.fg_tiles[index].0)
                            .map(|tile| tile_texture_properties_lookup[tile as usize].depth)
                            .map(|dep| (depth > dep) as u8)
                            .reduce(|acc, bit| (acc << 1) | bit)
                            .map(|out| (out << 2) as f32)
                            .unwrap();

                fg_tile_vertices.extend_from_slice(&[
                    TileVertex {
                        xyz: GlVec3(x - 8., y - 8., z),
                        uv: GlVec2(u, v),
                        mask_uv: GlVec2(mask_u, mask_v),
                    },
                    TileVertex {
                        xyz: GlVec3(x + tile_size_f32 + 8., y - 8., z),
                        uv: GlVec2(u + tile_size_f32, v),
                        mask_uv: GlVec2(mask_u + 4., mask_v),
                    },
                    TileVertex {
                        xyz: GlVec3(x + tile_size_f32 + 8., y + tile_size_f32 + 8., z),
                        uv: GlVec2(u + tile_size_f32, v + tile_size_f32),
                        mask_uv: GlVec2(mask_u + 4., mask_v + 4.),
                    },
                    TileVertex {
                        xyz: GlVec3(x - 8., y + tile_size_f32 + 8., z),
                        uv: GlVec2(u, v + tile_size_f32),
                        mask_uv: GlVec2(mask_u, mask_v + 4.),
                    },
                ]);

                // Skip check bg tile.
                continue;
            }

            // Skip BG if tile is None.
            if !matches!(game_render_desc.bg_tiles[index].0, TileKind::None) {
                let tile_texture_properties =
                    tile_texture_properties_lookup[game_render_desc.bg_tiles[index].0 as usize];

                // Get texture UV.
                let u = tile_texture_properties.u;
                let v = tile_texture_properties.v;

                // Get depth.
                let depth = tile_texture_properties.depth;

                // Calculate position.
                let x = tile_size_f32 * (x + game_render_desc.tiles_x) as f32;
                let y = tile_size_f32 * (y + game_render_desc.tiles_y) as f32;
                let z = depth as f32;

                // Calculate mask UV.
                #[rustfmt::skip]
                        let mask_u = [ index - stride, index - stride + 1, index + 1, index + stride + 1 ]
                            .into_iter()
                            .rev()
                            .map(|index| game_render_desc.bg_tiles[index].0)
                            .map(|tile| tile_texture_properties_lookup[tile as usize].depth)
                            .map(|dep| (depth > dep) as u8)
                            .reduce(|acc, bit| (acc << 1) | bit)
                            .map(|out| (out << 2) as f32)
                            .unwrap();
                #[rustfmt::skip]
                        let mask_v  = [index + stride, index + stride - 1, index - 1, index - stride - 1 ]
                            .into_iter()
                            .rev()
                            .map(|index| game_render_desc.bg_tiles[index].0)
                            .map(|tile| tile_texture_properties_lookup[tile as usize].depth)
                            .map(|dep| (depth > dep) as u8)
                            .reduce(|acc, bit| (acc << 1) | bit)
                            .map(|out| (out << 2) as f32)
                            .unwrap();

                bg_tile_vertices.extend_from_slice(&[
                    TileVertex {
                        xyz: GlVec3(x - 8., y - 8., z),
                        uv: GlVec2(u, v),
                        mask_uv: GlVec2(mask_u, mask_v),
                    },
                    TileVertex {
                        xyz: GlVec3(x + tile_size_f32 + 8., y - 8., z),
                        uv: GlVec2(u + tile_size_f32, v),
                        mask_uv: GlVec2(mask_u + 4., mask_v),
                    },
                    TileVertex {
                        xyz: GlVec3(x + tile_size_f32 + 8., y + tile_size_f32 + 8., z),
                        uv: GlVec2(u + tile_size_f32, v + tile_size_f32),
                        mask_uv: GlVec2(mask_u + 4., mask_v + 4.),
                    },
                    TileVertex {
                        xyz: GlVec3(x - 8., y + tile_size_f32 + 8., z),
                        uv: GlVec2(u, v + tile_size_f32),
                        mask_uv: GlVec2(mask_u, mask_v + 4.),
                    },
                ]);
            }
        }
    }

    (fg_tile_vertices, bg_tile_vertices)
}
