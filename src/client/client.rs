use crate::client::{GameFrame, GameUpdateState};
#[cfg(feature = "opengl")]
use crate::client::GameRenderState;
#[cfg(feature = "wgpu")]
use crate::client::GameRenderStateWgpu as GameRenderState;
use crate::time::*;
use crate::Window;
use std::path::Path;

pub struct Client {
    // Misc.
    server_port: u16,

    // Update.
    update_ts: u64,
    update_state: GameUpdateState,

    // Render.
    render_ts: u64,
    #[cfg(feature = "opengl")]
    render_state: GameRenderState,
    #[cfg(feature = "wgpu")]
    render_state: GameRenderState,

    // Diagnostic.
    acc_n: u64,
    prestep_acc: u64,
    step_acc: u64,
    poststep_acc: u64,
    render_acc: u64,
}

impl Client {
    pub fn new(root: &'static Path, server_port: u16, window: &mut Window) -> Self {
        #[cfg(feature = "opengl")]
        let render_state = GameRenderState::new(root);
        #[cfg(feature = "wgpu")]
        let render_state = pollster::block_on(GameRenderState::new(root, window)).unwrap();
        Self {
            server_port,

            update_ts: crate::timestamp_as_usecs(),
            update_state: GameUpdateState::new(root),

            render_ts: crate::timestamp_as_usecs(),
            render_state,

            acc_n: 0,
            prestep_acc: 0,
            step_acc: 0,
            poststep_acc: 0,
            render_acc: 0,
        }
    }

    fn handle_events(&mut self, events: &[crate::InputEvent], window: &mut Window) {
        for event in events {
            match event {
                crate::InputEvent::WindowResize(w, h) => {
                    #[cfg(feature = "wgpu")]
                    {
                        self.render_state.update_surface(window); 
                        self.render_state.resize((*w, *h));
                        // TODO: must rerender on mac here!
                        //self.render_state.render();
                        //window.swap_buffers();
                    }
                }
                _ => {}
            }
        }
    }

    pub fn update_once(&mut self, window: &mut Window) -> bool {
        let frametime = 16666_u64;

        //
        let next_timestamp = crate::timestamp_as_usecs();
        if next_timestamp - self.update_ts < frametime {
            return false;
        }

        // Get inputs.
        let input_events = window.poll();
        self.handle_events(&input_events, window);

        // Prestep.
        let ts = timestamp_as_usecs();
        {
            let end = self
                .update_state
                .prestep(self.update_ts, input_events.into_iter());
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
        if (self.acc_n > 60 * 5) {
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
