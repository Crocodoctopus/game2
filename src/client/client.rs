use crate::client::{GameRenderDesc, GameRenderState, GameUpdateState};
use crate::net::{ClientNetManager, NetEvent, NetEventKind};
use crate::shared::*;
use crate::time::*;
use crate::{InputEvent, Window};
use crossbeam_channel::Receiver;
use std::path::Path;

pub struct Client<'a> {
    // Misc.
    root: &'static Path,
    window: &'a Window,
    server_port: u16,

    // Update.
    input_events: Vec<InputEvent>,
    update_ts: u64,
    update_state: Option<GameUpdateState>,

    // Render.
    game_render_desc: Option<GameRenderDesc>,
    render_ts: u64,
    render_state: GameRenderState<'a>,

    // Diagnostic.
    update_n: u64,
    prestep_acc: u64,
    step_acc: u64,
    poststep_acc: u64,

    render_n: u64,
    render_acc: u64,
}

impl<'a> Client<'a> {
    pub fn new(root: &'static Path, window: &'a Window, server_port: u16) -> Self {
        Self {
            root,
            window,
            server_port,

            input_events: vec![],
            update_ts: 0,
            update_state: None,

            game_render_desc: None,
            render_ts: crate::timestamp_as_usecs(),
            render_state: GameRenderState::new(root, &window),

            update_n: 0,
            prestep_acc: 0,
            step_acc: 0,
            poststep_acc: 0,
            render_acc: 0,
            render_n: 0,
        }
    }

    pub fn run(mut self, input_recv: Receiver<InputEvent>) -> ! {
        let frametime = 16666_u64;

        // Start net manager.
        let server_dst = ("127.0.0.1", self.server_port);
        let mut net_manager = ClientNetManager::new(server_dst);

        // Connect/Join sequence.
        let (width, height, fg_tiles, bg_tiles) = 'start: {
            use crate::net::ClientNetSender;
            println!("[Client] Starting join sequence for {server_dst:?}.");

            // Send reliable Connect.
            net_manager.send_ru(serialize(&[ClientNetMessage::Connect { version: (0, 0) }]));

            // Output data.
            let mut world_w = 0;
            let mut world_h = 0;
            let mut fg_tiles = vec![]; //vec![Tile::None; world_w * world_h];
            let mut bg_tiles = vec![]; //vec![Tile::None; world_w * world_h];

            // Begin state sync.
            println!("[Client] Begin world construction sequence.");
            loop {
                // Wait for a net event.
                net_manager.poll();
                let net_events: Vec<NetEvent> = net_manager.recv().collect();
                for net_event in net_events {
                    match net_event {
                        // Data net events.
                        NetEvent {
                            source: _,
                            kind: NetEventKind::Data(bytes),
                        } => {
                            // Deserialize message.
                            for net_event in deserialize(bytes).into_vec() {
                                match net_event {
                                    // First message expected.
                                    ServerNetMessage::ConnectAccept => {
                                        net_manager.send_ru(serialize(&[ClientNetMessage::Join]));
                                    }

                                    // Second message expected.
                                    ServerNetMessage::WorldInfo { width, height } => {
                                        world_w = width as usize;
                                        world_h = height as usize;
                                        fg_tiles = vec![Tile::None; world_w * world_h];
                                        bg_tiles = vec![Tile::None; world_w * world_h];
                                    }

                                    // Lots of these are expected.
                                    ServerNetMessage::ChunkSync {
                                        x: inner_x,
                                        y: inner_y,
                                        seq: _,
                                        fg_tiles: inner_fg_tiles,
                                        bg_tiles: inner_bg_tiles,
                                    } => {
                                        let inner_x = inner_x as usize;
                                        let inner_y = inner_y as usize;
                                        for y in 0..CHUNK_SIZE {
                                            for x in 0..CHUNK_SIZE {
                                                fg_tiles[(inner_x + x) + (inner_y + y) * world_w] =
                                                    inner_fg_tiles[x + y * CHUNK_SIZE];
                                                bg_tiles[(inner_x + x) + (inner_y + y) * world_w] =
                                                    inner_bg_tiles[x + y * CHUNK_SIZE];
                                            }
                                        }
                                    }

                                    // Final message, escape the loop.
                                    ServerNetMessage::Start => {
                                        break 'start (
                                            world_w,
                                            world_h,
                                            fg_tiles.into_boxed_slice(),
                                            bg_tiles.into_boxed_slice(),
                                        );
                                    }

                                    _ => println!(
                                        "[Client] Unhandled event received during join sequence."
                                    ),
                                }
                            }
                        }

                        // Server booted us, probably for taking too long.
                        NetEvent {
                            source: _,
                            kind: NetEventKind::Disconnect,
                        } => {
                            panic!();
                        }

                        // Connect message, probably ignore it?
                        _ => {}
                    }
                }
            }
        };

        self.update_ts = crate::timestamp_as_usecs();
        self.update_state = Some(GameUpdateState::new(
            self.root, width, height, fg_tiles, bg_tiles,
        ));

        loop {
            // Record inputs.
            let input_events: Vec<InputEvent> = input_recv.try_iter().collect();
            self.input_events.extend(input_events.clone());

            // Run update "thread" if enough time has passed.
            let next_timestamp = crate::timestamp_as_usecs();
            if next_timestamp - self.update_ts >= frametime {
                let update_state = match &mut self.update_state {
                    Some(update_state) => update_state,
                    None => panic!(),
                };

                // Prestep.
                let ts = timestamp_as_usecs();
                {
                    // Extract net events.
                    net_manager.poll();
                    let net_events = net_manager.recv();

                    // Extract input events.
                    let input_events = std::mem::take(&mut self.input_events).into_iter();

                    // Prestep.
                    if update_state.prestep(self.update_ts, input_events, net_events) {
                        break;
                    }
                }
                self.prestep_acc += timestamp_as_usecs() - ts;

                // Step.
                let ts = timestamp_as_usecs();
                while self.update_ts + frametime <= next_timestamp {
                    update_state.step(self.update_ts, frametime);
                    self.update_ts += frametime;
                    self.update_n += 1;
                }
                self.step_acc += timestamp_as_usecs() - ts;

                // Poststep.
                let ts = timestamp_as_usecs();
                self.game_render_desc =
                    Some(update_state.poststep(self.update_ts, &mut net_manager));
                self.poststep_acc += timestamp_as_usecs() - ts;

                // Time printing.
                if self.update_n > 60 * 5 {
                    println!(
                    "[Client] Update total: {:.2}ms.\n  Prestep: {:.2}ms.\n  Step: {:.2}ms.\n  Poststep: {:.2}ms.",
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

            // Run render "thread".
            {
                let ts0 = timestamp_as_usecs();
                let mut ts1 = 0;
                self.render_state.handle_events(input_events.iter());
                if let Some(game_render_desc) = &self.game_render_desc {
                    ts1 = timestamp_as_usecs();
                    let surface = self.render_state.surface.get_current_texture().unwrap();
                    ts1 = timestamp_as_usecs() - ts1;
                    self.render_state
                        .render(surface, self.render_ts, game_render_desc);
                    self.render_n += 1;
                }
                self.render_ts += frametime;
                self.render_acc += timestamp_as_usecs() - ts0 - ts1;

                // Time printing.
                if self.render_n > 60 * 5 {
                    println!(
                        "[Client] Render total: {:.2}ms.",
                        (self.render_acc / self.render_n) as f32 * 0.001
                    );
                    self.render_acc = 0;
                    self.render_n = 0;
                }
            }
        }

        std::process::exit(0);
    }
}
