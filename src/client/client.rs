use crate::client::game_render_state::*;
use crate::client::game_update_state::*;
use crate::client::log;
use crate::net::{ClientNetManager, NetEventKind};
use crate::shared::net::*;
use crate::time::*;
use crate::window::*;
use crossbeam_channel::Receiver;
use std::path::Path;
use std::rc::Rc;

pub fn run_client(
    input_recv: Receiver<InputEvent>,
    root: &'static Path,
    window: Window,
    server_port: u16,
) -> ! {
    // Start net manager.
    let server_dst = ("127.0.0.1", server_port);
    let mut net_manager = ClientNetManager::new(server_dst);

    // Connect/wait.
    'start: {
        // Send reliable Connect.
        net_manager.send_ru(serialize(&[ClientNetMessage::Connect { version: (0, 0) }]));

        // Begin state sync.
        loop {
            // Wait for a net event.
            net_manager.poll();
            for net_event in net_manager.recv() {
                match net_event.kind {
                    // Data net events.
                    NetEventKind::Data(bytes) => {
                        // Deserialize message.
                        for net_event in deserialize(bytes).into_vec() {
                            match net_event {
                                // On ConnectAccept, allow client to do client things.
                                ServerNetMessage::ConnectAccept => {
                                    break 'start;
                                }

                                _ => log!("Unhandled event received during join sequence."),
                            }
                        }
                    }

                    // Server booted us, probably for taking too long.
                    NetEventKind::Disconnect => panic!(),

                    // Connect message, probably ignore it?
                    _ => {}
                }
            }
        }
    };

    // Set up game loop and start.
    let mut update_input_events = vec![];
    let mut render_input_events = vec![];

    let mut game_render_desc = None;
    let mut render_ts = crate::timestamp_as_usecs();
    let (context, surface) = gl_create_context(&window);
    let mut render_state = GameRenderState::new(root, context, surface);

    let mut update_n = 0;
    let mut prestep_acc = 0;
    let mut step_acc = 0;
    let mut poststep_acc = 0;
    let mut render_acc = 0;
    let mut render_n = 0;

    let frametime = 16666_u64;
    let mut update_ts = crate::timestamp_as_usecs();
    let mut update_state = Some(GameUpdateState::new(root, net_manager));

    loop {
        // Record inputs.
        let input_events: Vec<InputEvent> = input_recv.try_iter().collect();
        update_input_events.extend(input_events.clone());
        render_input_events.extend(input_events.clone());
        drop(input_events);

        // Run update "thread" if enough time has passed.
        let next_timestamp = crate::timestamp_as_usecs();
        if next_timestamp - update_ts >= frametime {
            let update_state: &mut GameUpdateState = match update_state.as_mut() {
                Some(update_state) => update_state,
                None => panic!(),
            };

            // Prestep.
            let ts = timestamp_as_usecs();
            {
                // Extract input events.
                let input_events = std::mem::take(&mut update_input_events).into_iter();

                // Prestep.
                if update_state.prestep(update_ts, input_events) {
                    break;
                }
            }
            prestep_acc += timestamp_as_usecs() - ts;

            // Step.
            let ts = timestamp_as_usecs();
            while update_ts + frametime <= next_timestamp {
                update_state.step(update_ts, frametime);
                update_ts += frametime;
                update_n += 1;
            }
            step_acc += timestamp_as_usecs() - ts;

            // Poststep.
            let ts = timestamp_as_usecs();
            game_render_desc = Some(update_state.poststep(update_ts));
            poststep_acc += timestamp_as_usecs() - ts;

            // Time printing.
            if update_n > 60 * 30 {
                log!(
                    "Update total: {:.2}ms.\n  Prestep: {:.2}ms.\n  Step: {:.2}ms.\n  Poststep: {:.2}ms.",
                    ((prestep_acc + step_acc + poststep_acc)
                        / update_n) as f32 * 0.001,
                    (prestep_acc / update_n) as f32 * 0.001,
                    (step_acc / update_n) as f32 * 0.001,
                    (poststep_acc / update_n) as f32 * 0.001,
                );
                prestep_acc = 0;
                step_acc = 0;
                poststep_acc = 0;
                update_n = 0;
            }
        }

        // Run render "thread".
        {
            let ts0 = timestamp_as_usecs();
            let mut ts1 = 0;
            let input_events = std::mem::take(&mut render_input_events).into_iter();
            render_state.handle_events(input_events);
            if let Some(game_render_desc) = &game_render_desc {
                ts1 = timestamp_as_usecs();
                //let surface = render_state.surface.get_current_texture().unwrap();
                ts1 = timestamp_as_usecs() - ts1;
                render_state.render(render_ts, game_render_desc);
                render_n += 1;
            }
            render_ts += frametime;
            render_acc += timestamp_as_usecs() - ts0 - ts1;

            //window.swap();

            // Time printing.
            if render_n > 60 * 30 {
                log!(
                    "Render total: {:.2}ms.",
                    (render_acc / render_n) as f32 * 0.001
                );
                render_acc = 0;
                render_n = 0;
            }
        }
    }

    std::process::exit(0);
}

/*
pub struct Client {
    // Misc.
    root: &'static Path,
    #[allow(dead_code)]
    window: Window,
    server_port: u16,

    // Update.
    input_events: Vec<InputEvent>,
    update_ts: u64,
    update_state: Option<GameUpdateState>,

    // Render.
    game_render_desc: Option<GameRenderDesc>,
    render_ts: u64,
    render_state: GameRenderState,

    // Diagnostic.
    update_n: u64,
    prestep_acc: u64,
    step_acc: u64,
    poststep_acc: u64,

    render_n: u64,
    render_acc: u64,
}

impl Client {
    pub fn new(root: &'static Path, window: Window, server_port: u16) -> Self {
        let (context, surface) = gl_create_context(&window);
        Self {
            root,
            window,
            server_port,

            input_events: vec![],
            update_ts: 0,
            update_state: None,

            game_render_desc: None,
            render_ts: crate::timestamp_as_usecs(),
            render_state: GameRenderState::new(root, context, surface),

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

        // Connect/wait.
        'start: {
            // Send reliable Connect.
            net_manager.send_ru(serialize(&[ClientNetMessage::Connect { version: (0, 0) }]));

            // Begin state sync.
            loop {
                // Wait for a net event.
                net_manager.poll();
                for net_event in net_manager.recv() {
                    match net_event.kind {
                        // Data net events.
                        NetEventKind::Data(bytes) => {
                            // Deserialize message.
                            for net_event in deserialize(bytes).into_vec() {
                                match net_event {
                                    // On ConnectAccept, allow client to do client things.
                                    ServerNetMessage::ConnectAccept => {
                                        break 'start;
                                    }

                                    _ => log!("Unhandled event received during join sequence."),
                                }
                            }
                        }

                        // Server booted us, probably for taking too long.
                        NetEventKind::Disconnect => panic!(),

                        // Connect message, probably ignore it?
                        _ => {}
                    }
                }
            }
        };

        self.update_ts = crate::timestamp_as_usecs();
        self.update_state = Some(GameUpdateState::new(self.root, net_manager));

        loop {
            // Record inputs.
            let input_events: Vec<InputEvent> = input_recv.try_iter().collect();
            self.input_events.extend(input_events.clone());

            // Run update "thread" if enough time has passed.
            let next_timestamp = crate::timestamp_as_usecs();
            if next_timestamp - self.update_ts >= frametime {
                let update_state: &mut GameUpdateState = match self.update_state.as_mut() {
                    Some(update_state) => update_state,
                    None => panic!(),
                };

                // Prestep.
                let ts = timestamp_as_usecs();
                {
                    // Extract input events.
                    let input_events = std::mem::take(&mut self.input_events).into_iter();

                    // Prestep.
                    if update_state.prestep(self.update_ts, input_events) {
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
                self.game_render_desc = Some(update_state.poststep(self.update_ts));
                self.poststep_acc += timestamp_as_usecs() - ts;

                // Time printing.
                if self.update_n > 60 * 30 {
                    log!(
                    "Update total: {:.2}ms.\n  Prestep: {:.2}ms.\n  Step: {:.2}ms.\n  Poststep: {:.2}ms.",
                    ((self.prestep_acc + self.step_acc + self.poststep_acc)
                        / self.update_n) as f32 * 0.001,
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
                    //let surface = self.render_state.surface.get_current_texture().unwrap();
                    ts1 = timestamp_as_usecs() - ts1;
                    self.render_state.render(self.render_ts, game_render_desc);
                    self.render_n += 1;
                }
                self.render_ts += frametime;
                self.render_acc += timestamp_as_usecs() - ts0 - ts1;

                // Time printing.
                if self.render_n > 60 * 30 {
                    log!(
                        "Render total: {:.2}ms.",
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
*/
