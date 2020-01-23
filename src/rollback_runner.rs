use crate::game::{GameInput, GameState};
use crate::input_history::InputHistory;
use crate::net_client::TestNetClient;
use ggez::event::EventHandler;
use ggez::event::{KeyCode, KeyMods};
use ggez::{graphics, Context, GameResult};
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::time::Instant;

pub struct RollbackRunner {
    current_state: GameState,
    p1_input: InputHistory<GameInput>,
    p2_input: InputHistory<GameInput>,
    input_state: i32,
    player1: bool,
    client: TestNetClient,
    current_frame: i32,
    recieved_inputs: Vec<InputTiming>,
    start_time: Instant,
    skip_frames: i32,
    ping: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InputTiming {
    frame: i32,
    input: GameInput,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct InputData {
    frame: i32,
    input: InputTiming,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum RollbackPacket {
    Ping(u128),
    Pong(u128),
    Input(InputData),
    Request(i32),
    Provide(InputTiming),
}

impl RollbackRunner {
    pub fn new(ctx: &mut Context, player1: bool, client: TestNetClient) -> RollbackRunner {
        // Load/create resources such as images here.
        RollbackRunner {
            current_state: GameState::new(ctx),
            p1_input: InputHistory::new(GameInput { x_axis: 0 }),
            p2_input: InputHistory::new(GameInput { x_axis: 0 }),
            input_state: 0,
            player1,
            client,
            recieved_inputs: Vec::new(),
            start_time: Instant::now(),
            current_frame: 0,
            skip_frames: 0,
            ping: 0.0,
        }
    }
}

fn ping_in_delay(value: f32) -> i32 {
    ((value + 3.0) / 32.0).ceil() as i32
}

impl EventHandler for RollbackRunner {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let start_time = Instant::now();
        let current_time = (start_time - self.start_time).as_millis();
        let (local_player, net_player) = if self.player1 {
            (&mut self.p1_input, &mut self.p2_input)
        } else {
            (&mut self.p2_input, &mut self.p1_input)
        };
        if !net_player.has_input(self.current_frame) {
            self.client
                .send(&RollbackPacket::Request(self.current_frame))
                .unwrap();
            dbg!("missing net sides ");
        }

        'poll_packets: loop {
            match self.client.recv::<RollbackPacket>() {
                Ok(packet) => match packet {
                    RollbackPacket::Ping(ping_time) => {
                        self.client.send(&RollbackPacket::Pong(ping_time)).unwrap();
                    }
                    RollbackPacket::Pong(pong_time) => {
                        let ping_time = current_time - pong_time;
                        self.ping = self.ping * 0.9 + ping_time as f32 * 0.1;
                    }
                    RollbackPacket::Input(input) => {
                        // this is for calculating how many frames to skip
                        self.skip_frames =
                            self.current_frame - (input.frame + ping_in_delay(self.ping));
                        self.recieved_inputs.push(input.input);
                    }

                    RollbackPacket::Request(frame) => {
                        if let Some(input) = local_player.get_input(frame) {
                            let input = input.clone();
                            self.client
                                .send(&RollbackPacket::Provide(InputTiming { frame, input }))
                                .unwrap();
                        }
                    }
                    RollbackPacket::Provide(requested) => {
                        self.recieved_inputs.push(requested);
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
                dbg!("skipped a frame!");
            } else {
                local_player.add_local_input(
                    self.current_frame,
                    GameInput {
                        x_axis: self.input_state,
                    },
                );
                self.client
                    .send(&RollbackPacket::Input(InputData {
                        input: InputTiming {
                            input: GameInput {
                                x_axis: self.input_state,
                            },
                            frame: self.current_frame,
                        },
                        frame: self.current_frame,
                    }))
                    .unwrap();
                let start_time = Instant::now();
                let current_time = (start_time - self.start_time).as_millis();
                self.client
                    .send(&RollbackPacket::Ping(current_time))
                    .unwrap();

                self.recieved_inputs.sort_by(|l, r| l.frame.cmp(&r.frame));
                for input in self.recieved_inputs.drain(..) {
                    net_player.add_network_input(input.frame, input.input);
                }

                if self.p1_input.has_input(self.current_frame)
                    && self.p2_input.has_input(self.current_frame)
                {
                    self.current_state.update(
                        &self.p1_input.get_input(self.current_frame).unwrap(),
                        &self.p2_input.get_input(self.current_frame).unwrap(),
                    );
                    self.current_frame += 1;
                }
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
        self.current_state.draw(ctx, 100.0)?;
        graphics::present(ctx)
    }
}
