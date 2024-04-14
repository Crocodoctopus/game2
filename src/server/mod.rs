pub mod game_update_state;
pub mod server;

pub use game_update_state::*;
pub use server::*;

pub mod server_log {
    macro_rules! log {
        ($($args:tt)*) => {{
            println!("\x1b[91m[Server] {}\x1b[0m", format!($($args)*));
        }}
    }

    pub(crate) use log;
}

use server_log::log;
