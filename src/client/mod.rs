pub mod client;
pub mod game_render_desc;
pub mod game_render_state;
pub mod game_update_state;

pub mod client_log {
    macro_rules! log {
        ($($args:tt)*) => {{
            println!("\x1b[92m[Client] {}\x1b[0m", format!($($args)*));
        }}
    }

    pub(crate) use log;
}

use client_log::log;
