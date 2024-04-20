use crate::net::ServerNetManager;
use crate::server::GameUpdateState;
use crate::time::timestamp_as_usecs;
use std::path::Path;

pub struct Server {
    // Update.
    update_ts: u64,
    update_state: GameUpdateState,

    // Diagnostic.
    update_n: u64,
    prestep_acc: u64,
    step_acc: u64,
    poststep_acc: u64,
}

impl Server {
    pub fn new(root: &'static Path, bind_port: u16) -> (Self, u16) {
        let (net_manager, bind_port) = ServerNetManager::new(bind_port);

        (
            Self {
                update_ts: timestamp_as_usecs(),
                update_state: GameUpdateState::new(root, net_manager),

                update_n: 0,
                prestep_acc: 0,
                step_acc: 0,
                poststep_acc: 0,
            },
            bind_port,
        )
    }

    pub fn run(mut self) -> ! {
        let frametime = 33333_u64;

        //
        loop {
            // Wait for enough time to process a frame.
            let next_timestamp = crate::time::wait(self.update_ts + frametime, 1_000);
            assert!(next_timestamp - self.update_ts >= frametime,);

            // Prestep.
            let ts = timestamp_as_usecs();
            {
                self.update_state.prestep(self.update_ts);
            }
            self.prestep_acc += timestamp_as_usecs() - ts;

            // Step.
            let ts = timestamp_as_usecs();
            {
                while self.update_ts + frametime <= next_timestamp {
                    self.update_state.step(self.update_ts, frametime);
                    self.update_ts += frametime;
                    self.update_n += 1;
                }
            }
            self.step_acc += timestamp_as_usecs() - ts;

            // Step.
            let ts = timestamp_as_usecs();
            {
                self.update_state.poststep(self.update_ts);
            }
            self.poststep_acc += timestamp_as_usecs() - ts;

            // Time printing.
            if self.update_n > 60 * 30 {
                println!(
                    "\x1b[91m[Server] Update total: {:.2}ms.\n  Prestep: {:.2}ms.\n  Step: {:.2}ms.\n  Poststep: {:.2}ms.\x1b[0m",
                    ((self.prestep_acc + self.step_acc + self.poststep_acc)
                        / self.update_n) as f32
                        * 0.001,
                    (self.prestep_acc / self.update_n) as f32 * 0.001,
                    (self.step_acc / self.update_n) as f32 * 0.001,
                    (self.poststep_acc / self.update_n) as f32 * 0.001,
                );
                self.prestep_acc = 0;
                self.step_acc = 0;
                self.poststep_acc = 0;
                self.update_n = 0;
            }
        }
    }
}
