use lazy_static::lazy_static;

use std::time::Instant;

lazy_static! {
    // Timestamp since the start of the program.
    pub static ref TIMESTAMP: Instant = Instant::now();
}

pub fn timestamp_as_usecs() -> u64 {
    Instant::now().duration_since(*TIMESTAMP).as_micros() as u64
}

pub fn timestamp_as_msecs() -> u64 {
    Instant::now().duration_since(*TIMESTAMP).as_millis() as u64
}

pub fn timestamp_as_secs() -> u64 {
    Instant::now().duration_since(*TIMESTAMP).as_secs()
}
