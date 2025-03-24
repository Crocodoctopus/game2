mod client;
mod gen_map;
mod net;
mod server;
mod shared;
mod time;
mod window;

use crate::client::client::run_client;
use crate::net::ServerNetManager;
use crate::server::server::run_server;
use crate::window::{EventLoop, Window};
use once_cell::sync::Lazy;
use std::path::PathBuf;
use time::timestamp_as_usecs;

fn main() {
    // Initialize game start timestamp.
    lazy_static::initialize(&time::TIMESTAMP);

    // Get working directory.
    static PATH: Lazy<PathBuf> = Lazy::new(|| {
        std::env::current_exe()
            .expect("Could not get CWD.")
            .parent()
            .unwrap()
            .to_owned()
    });

    // Ceate event loop.
    let event_loop = winit::event_loop::EventLoop::new().unwrap();

    // Create window.
    let window_attributes = winit::window::WindowAttributes::default()
        .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720))
        .with_resizable(false);
    let (window, gl_config) = glutin_winit::DisplayBuilder::default()
        .with_window_attributes(Some(window_attributes))
        .build(
            &event_loop,
            glutin::config::ConfigTemplateBuilder::default(),
            |mut configs| configs.next().unwrap(),
        )
        .unwrap();
    let window = window.unwrap();

    // Wrappers.
    let event_loop = EventLoop::new(event_loop);
    let window = Window::new(window, gl_config);

    // Initialize net manager.
    let (net_manager, port) = ServerNetManager::new(0);

    // Start.
    let (input_send, input_recv) = crossbeam_channel::bounded(10);
    std::thread::scope(|s| {
        let _client_thread = s.spawn(|| run_client(input_recv, &PATH, window, port));
        let _server_thread = s.spawn(|| run_server(&PATH, net_manager));
        event_loop.run(|event| input_send.send(event).unwrap());
    })
}
