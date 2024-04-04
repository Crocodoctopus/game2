use crate::client::Client;
use crate::gl;
use crate::gl::types::*;
use crate::server::Server;
use crate::Window;
use std::path::Path;

pub struct ClientApp {
    // Misc.
    root: &'static Path,

    // Window.
    glfw: glfw::Glfw,
    window: Window,

    // Client/server.
    client: Client,
    server: Server,
}

impl ClientApp {
    pub fn new(root: &'static Path) -> Self {
        // Init GLFW.
        let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();

        // Create window.
        let mut window = Window::new(&mut glfw, 1280, 720);

        // Load GL functions.
        window.gl_load();

        // Print GPU info.
        unsafe {
            use std::ffi::CStr;
            let vendor = CStr::from_ptr(gl::GetString(gl::VENDOR) as *const i8);
            let renderer = CStr::from_ptr(gl::GetString(gl::RENDERER) as *const i8);
            println!("GPU in use: [{vendor:?}] {renderer:?}.");
        }

        // Initialize server.
        let (server, port) = Server::new(0);

        // Initialize client.
        let client = Client::new(root, port);

        Self {
            root,

            glfw,
            window,

            client,
            server,
        }
    }

    pub fn run(&mut self) {
        loop {
            // Kinda hacky.
            self.glfw.poll_events();

            // Update client.
            let brk = self.client.update_once(&mut self.window);
            if brk {
                break;
            }

            // Swap.
            self.window.swap();

            // Update server.
            self.server.update_once();
        }
    }
}
