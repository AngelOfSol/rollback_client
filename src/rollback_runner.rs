use crate::game::{GameInput, GameState};
use crate::net_client::TestNetClient;
use ggez::event::EventHandler;
use ggez::event::{KeyCode, KeyMods};
use ggez::{graphics, Context, GameResult};
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::time::Instant;

pub struct RollbackRunner {
    current_state: Vec<GameState>,
    p1_input: Vec<GameInput>,
    p2_input: Vec<GameInput>,
    input_state: i32,
    player1: bool,
    client: TestNetClient,
    current_frame: i32,
    recently_recieved_packets: Vec<RollbackPacket>,
    start_time: Instant,
    last_recieved_frame: i32,
    skip_frames: i32,
    ping: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum RollbackPacket {
    Ping(u128),
    Pong(u128),
    Input { frame: i32, input: GameInput },
}

impl RollbackRunner {
    pub fn new(ctx: &mut Context, player1: bool, client: TestNetClient) -> RollbackRunner {
        // Load/create resources such as images here.
        RollbackRunner {
            current_state: vec![GameState::new(ctx)],
            p1_input: vec![GameInput { x_axis: 0 }],
            p2_input: vec![GameInput { x_axis: 0 }],
            input_state: 0,
            player1,
            client,
            recently_recieved_packets: Vec::new(),
            start_time: Instant::now(),
            current_frame: 0,
            last_recieved_frame: 0,
            skip_frames: 0,
            ping: 0.0,
        }
    }
}

impl EventHandler for RollbackRunner {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let start_time = Instant::now();
        let current_time = (start_time - self.start_time).as_millis();
        let (target_player, other_player) = if self.player1 {
            (&mut self.p1_input, &mut self.p2_input)
        } else {
            (&mut self.p2_input, &mut self.p1_input)
        };

        'poll_packets: loop {
            match self.client.recv::<RollbackPacket>() {
                Ok(packet) => match packet {
                    RollbackPacket::Ping(ping_time) => {
                        self.client.send(&RollbackPacket::Pong(ping_time)).unwrap();
                    }
                    RollbackPacket::Pong(pong_time) => {
                        let ping_time = (current_time - pong_time) / 2;
                        self.ping = self.ping * 0.9 + ping_time as f32 * 0.1;
                    }
                    input @ RollbackPacket::Input { .. } => {
                        if let RollbackPacket::Input { ref frame, .. } = input {
                            // this is for calculating how many frames to skip
                            let ping_in_frames = (self.ping / 16.0) as i32 + 1;
                            self.skip_frames = self.current_frame - (frame + ping_in_frames);
                            self.last_recieved_frame = *frame;
                        }
                        self.recently_recieved_packets.push(input.clone());
                    }
                },
                Err(e) if e.kind() == ErrorKind::WouldBlock => break 'poll_packets,
                Err(e) => {
                    panic!("{:?}", e);
                }
            }
        }
        if ggez::timer::check_update_time(ctx, 60) {
            if self.skip_frames > 0 {
                self.skip_frames -= 1;
            /*
            dbg!("skipped a frame");
            dbg!(self.ping);
            dbg!(self.last_recieved_frame);
            dbg!(self.current_frame);
            dbg!(self.skip_frames);*/
            } else {
                self.client
                    .send(&RollbackPacket::Input {
                        input: GameInput {
                            x_axis: self.input_state,
                        },
                        frame: self.current_frame,
                    })
                    .unwrap();
                let start_time = Instant::now();
                let current_time = (start_time - self.start_time).as_millis();
                self.client
                    .send(&RollbackPacket::Ping(current_time))
                    .unwrap();
                target_player.insert(
                    0,
                    GameInput {
                        x_axis: self.input_state,
                    },
                );

                if !self.p1_input.is_empty() && !self.p2_input.is_empty() {
                    self.current_state[0].update(
                        &self.p1_input[0..10.min(self.p1_input.len())],
                        &self.p2_input[0..10.min(self.p2_input.len())],
                    );
                }
                self.current_frame += 1;
            }
        }
        self.client.send_queued()?;
        Ok(())
    }
    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymod: KeyMods,
        repeat: bool,
    ) {
        if !repeat {
            self.input_state = match keycode {
                KeyCode::Left => -1,
                KeyCode::Right => 1,
                _ => 0,
            };
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: KeyCode, _keymod: KeyMods) {
        self.input_state = match keycode {
            KeyCode::Left => 0,
            KeyCode::Right => 0,
            _ => 0,
        };
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);
        self.current_state[0].draw(ctx, 100.0)?;
        graphics::present(ctx)
    }
}
