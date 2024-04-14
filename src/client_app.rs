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

        let (input_send, input_recv) = crossbeam_channel::bounded(100);

        // Initialize server.
        let (mut server, port) = Server::new(root, 0);

        // Initialize client.
        let mut client = Client::new(root, &window, port);

        // Start.
        std::thread::scope(|s| {
            let client_thread = s.spawn(|| client.run(input_recv));
            let server_thread = s.spawn(|| server.run());
            event_loop.run(|event| {
                println!("{event:?}");
                input_send.send(event).unwrap();
            });
        })
    }
}
