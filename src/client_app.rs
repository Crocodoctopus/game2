use crate::client::Client;
use crate::server::Server;
use crate::{EventLoop, Window};
use std::path::Path;

pub struct ClientApp<'a> {
    // Misc.
    root: &'static Path,

    // Window.
    window: Window,

    // Client/server.
    client: Client<'a>,
    server: Server,
}

impl<'a> ClientApp<'a> {
    pub fn launch(root: &'static Path) -> ! {
        // Window.
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720))
            .with_resizable(false)
            .build(&event_loop)
            .unwrap();

        // Wrappers.
        let mut event_loop = EventLoop::new(event_loop);
        let window = Window::new(window);

        // Initialize server.
        let (mut server, port) = Server::new(0);

        // Initialize client.
        let mut client = Client::new(root, port, &window);

        'end: loop {
            // Update server.
            server.update_once();

            // Poll events.
            let input_events = event_loop.poll();

            // Update client.
            let brk = client.update_once(&window, input_events);
            if brk {
                break;
            }

            // Swap.
            window.swap();
        }

        std::process::exit(0);
    }
}
