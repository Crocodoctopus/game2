use lazy_static::lazy_static;

use std::time::{Duration, Instant};

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

pub fn wait(time: u64, buffer: u64) -> u64 {
    if (time < timestamp_as_usecs()) {
        return timestamp_as_usecs();
    }

    // Sleep for the duration, with a buffer
    std::thread::sleep(Duration::from_micros(
        time.saturating_sub(timestamp_as_usecs())
            .saturating_sub(buffer),
    ));

    // Spin for the remaining time
    while timestamp_as_usecs() < time {
        std::hint::spin_loop();
        std::thread::yield_now();
    }

    // Return the current time, which should be close to ``time``
    return timestamp_as_usecs();
}
