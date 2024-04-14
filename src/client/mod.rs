pub mod client;
mod game_render_desc;
mod game_render_state;
mod game_update_state;

pub use client::*;
use game_render_desc::*;
use game_render_state::*;
use game_update_state::*;

pub mod client_log {
    macro_rules! log {
        ($($args:tt)*) => {{
            println!("\x1b[92m[Client] {}\x1b[0m", format!($($args)*));
        }}
    }

    pub(crate) use log;
}

use client_log::log;
