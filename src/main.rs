mod client;
mod client_app;
mod net;
mod server;
mod shared;
mod time;
mod window;

pub use shared::*;
pub use time::{timestamp_as_msecs, timestamp_as_secs, timestamp_as_usecs};
pub use window::*;

use crate::client_app::ClientApp;
use once_cell::sync::Lazy;
use std::path::PathBuf;

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

    // Start/Run/Free app.
    ClientApp::launch(&PATH);
}
