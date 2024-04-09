use crate::client::{GameRenderState, GameUpdateState};
use crate::time::*;
use crate::{InputEvent, Window};
use std::path::Path;

pub struct Client<'a> {
    // Misc.
    server_port: u16,

    // Update.
    update_ts: u64,
    update_state: GameUpdateState,

    // Render.
    render_ts: u64,
    render_state: GameRenderState<'a>,

    // Diagnostic.
    acc_n: u64,
    prestep_acc: u64,
    step_acc: u64,
    poststep_acc: u64,
    render_acc: u64,
}

impl<'a> Client<'a> {
    pub fn new(root: &'static Path, server_port: u16, window: &'a Window) -> Self {
        Self {
            server_port,

            update_ts: crate::timestamp_as_usecs(),
            update_state: GameUpdateState::new(root),

            render_ts: crate::timestamp_as_usecs(),
            render_state: GameRenderState::new(root, window),

            acc_n: 0,
            prestep_acc: 0,
            step_acc: 0,
            poststep_acc: 0,
            render_acc: 0,
        }
    }

    pub fn update_once(
        &mut self,
        window: &'a Window,
        input_events: impl Iterator<Item = InputEvent>,
    ) -> bool {
        let frametime = 16666_u64;

        //
        let next_timestamp = crate::timestamp_as_usecs();
        if next_timestamp - self.update_ts < frametime {
            return false;
        }

        // Prestep.
        let ts = timestamp_as_usecs();
        {
            let end = self.update_state.prestep(self.update_ts, input_events);
            if end {
                return true;
            }
        }
        self.prestep_acc += timestamp_as_usecs() - ts;

        // Step.
        let ts = timestamp_as_usecs();
        while self.update_ts + frametime <= next_timestamp {
            self.update_state.step(self.update_ts, frametime);
            self.update_ts += frametime;
        }
        self.step_acc += timestamp_as_usecs() - ts;

        // Poststep.
        let ts = timestamp_as_usecs();
        let game_frame = self.update_state.poststep(self.update_ts);
        self.poststep_acc += timestamp_as_usecs() - ts;

        // Render.
        let ts = timestamp_as_usecs();
        self.render_state.render(self.render_ts, game_frame);
        self.render_ts += frametime;
        self.render_acc += timestamp_as_usecs() - ts;

        self.acc_n += 1;
        if self.acc_n > 60 * 5 {
            println!(
                "Frame: {:.2}ms.\n  Prestep: {:.2}ms.\n  Step: {:.2}ms.\n  Poststep: {:.2}ms.\n  Render: {:.2}ms.",
                ((self.prestep_acc + self.step_acc + self.poststep_acc + self.render_acc)
                    / self.acc_n) as f32
                    * 0.001,
                (self.prestep_acc / self.acc_n) as f32 * 0.001,
                (self.step_acc / self.acc_n) as f32 * 0.001,
                (self.poststep_acc / self.acc_n) as f32 * 0.001,
                (self.render_acc / self.acc_n) as f32 * 0.001
            );
            self.prestep_acc = 0;
            self.step_acc = 0;
            self.poststep_acc = 0;
            self.render_acc = 0;
            self.acc_n = 0;
        }

        return false;
    }
}
