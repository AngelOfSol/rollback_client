use crate::game::{GameInput, GameState};
use crate::net_client::TestNetClient;
use crate::netcode::{self, Action, NetcodeClient};
use ggez::event::EventHandler;
use ggez::event::{KeyCode, KeyMods};
use ggez::{graphics, Context, GameResult};
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::time::Instant;

pub struct RollbackRunner {
    current_state: GameState,
    delay_client: NetcodeClient<GameInput>,
    input_state: i32,
    player1: bool,
    client: TestNetClient,
    start_time: Instant,
    dropped: Vec<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InputTiming {
    frame: i32,
    input: GameInput,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct InputData {
    frame: i32,
    input: Vec<InputTiming>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum RollbackPacket {
    Ping(u128),
    Pong(u128),
    Netcode(netcode::Packet<GameInput>),
}

impl RollbackRunner {
    pub fn new(ctx: &mut Context, player1: bool, client: TestNetClient) -> RollbackRunner {
        // Load/create resources such as images here.
        RollbackRunner {
            current_state: GameState::new(ctx),
            delay_client: NetcodeClient::new(10),
            input_state: 0,
            player1,
            client,
            start_time: Instant::now(),
            dropped: vec![false; 300],
        }
    }
}

impl EventHandler for RollbackRunner {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let start_time = Instant::now();
        let current_time = (start_time - self.start_time).as_millis();

        let mut count = 0;
        'poll_packets: loop {
            match self.client.recv::<RollbackPacket>() {
                Ok(packet) => match packet {
                    RollbackPacket::Ping(ping_time) => {
                        self.client.send(&RollbackPacket::Pong(ping_time)).unwrap();
                    }
                    RollbackPacket::Pong(pong_time) => {
                        let ping_time = current_time - pong_time;
                        self.delay_client.ping =
                            self.delay_client.ping * 0.9 + ping_time as f32 * 0.1;
                    }
                    RollbackPacket::Netcode(input) => {
                        // this is for calculating how many frames to skip
                        if let Some(packet) = self.delay_client.handle_packet(input) {
                            self.client.send(&RollbackPacket::Netcode(packet)).unwrap();
                        };
                    }
                },
                Err(e) if e.kind() == ErrorKind::WouldBlock => break 'poll_packets,
                Err(e) => {
                    panic!("{:?}", e);
                }
            }
            count += 1;
            if count % 20 == 0 {
                dbg!(count);
            }
        }

        let fps = 60;
        if ggez::timer::check_update_time(ctx, fps) {
            self.client
                .send(&RollbackPacket::Netcode(
                    self.delay_client.handle_local_input(GameInput {
                        x_axis: self.input_state,
                    }),
                ))
                .unwrap();

            let start_time = Instant::now();
            let current_time = (start_time - self.start_time).as_millis();
            self.client
                .send(&RollbackPacket::Ping(current_time))
                .unwrap();

            match self.delay_client.idle() {
                Action::DoNothing => {}
                Action::Request(packet) => {
                    self.client.send(&RollbackPacket::Netcode(packet)).unwrap();
                }
                Action::RunInput(input) => {
                    if self.player1 {
                        self.current_state
                            .update(&input.local.last().unwrap(), &input.net.last().unwrap());
                    } else {
                        self.current_state
                            .update(&input.net.last().unwrap(), &input.local.last().unwrap());
                    }
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
            match keycode {
                KeyCode::D => self.client.delay += std::time::Duration::from_millis(10),
                KeyCode::A => {
                    if self.client.delay >= std::time::Duration::from_millis(10) {
                        self.client.delay -= std::time::Duration::from_millis(10)
                    }
                }
                KeyCode::W => self.client.packet_loss += 0.05,
                KeyCode::S => self.client.packet_loss -= 0.05,
                KeyCode::E => self.delay_client.TEMP_buffer_size += 1,
                KeyCode::Q => {
                    self.delay_client.TEMP_buffer_size -= 1;
                    self.delay_client.TEMP_buffer_size = self.delay_client.TEMP_buffer_size.max(1)
                }

                KeyCode::Z => self.delay_client.TEMP_additional_input_delay -= 1,
                KeyCode::C => self.delay_client.TEMP_additional_input_delay += 1,
                _ => (),
            };

            self.client.packet_loss = self.client.packet_loss.max(0.0).min(1.0);
            self.client.delay = self
                .client
                .delay
                .max(std::time::Duration::from_millis(0))
                .min(std::time::Duration::from_millis(100));
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
        graphics::draw(
            ctx,
            &graphics::Text::new(format!("Delay: {:.2}f", self.delay_client.input_delay())),
            graphics::DrawParam::default().dest([30.0, 200.0]),
        )?;
        graphics::draw(
            ctx,
            &graphics::Text::new(format!("Ping (ms): {:.2}", self.delay_client.ping / 2.0)),
            graphics::DrawParam::default().dest([30.0, 250.0]),
        )?;
        graphics::draw(
            ctx,
            &graphics::Text::new(format!(
                "Current Frame: f{:.2}",
                self.delay_client.current_frame()
            )),
            graphics::DrawParam::default().dest([30.0, 300.0]),
        )?;
        graphics::draw(
            ctx,
            &graphics::Text::new(format!(
                "Dropped Frame (%): {:.2}",
                self.dropped.iter().filter(|x| **x).count() as f32 / self.dropped.len() as f32
                    * 100.0
            )),
            graphics::DrawParam::default().dest([30.0, 350.0]),
        )?;
        graphics::draw(
            ctx,
            &graphics::Text::new(format!(
                "Dropped Frame (per second): {:.0}",
                self.dropped.iter().filter(|x| **x).count() as f32 / self.dropped.len() as f32
                    * 60.0
            )),
            graphics::DrawParam::default().dest([30.0, 400.0]),
        )?;

        graphics::draw(
            ctx,
            &graphics::Text::new(format!(
                "Faked Network Delay (ms): {:.2}",
                self.client.delay.as_millis()
            )),
            graphics::DrawParam::default().dest([300.0, 200.0]),
        )?;
        graphics::draw(
            ctx,
            &graphics::Text::new(format!(
                "Faked Packet Loss (%): {:.2}",
                self.client.packet_loss * 100.0
            )),
            graphics::DrawParam::default().dest([300.0, 250.0]),
        )?;
        graphics::draw(
            ctx,
            &graphics::Text::new(format!(
                "Buffer Size (f): {}",
                self.delay_client.TEMP_buffer_size,
            )),
            graphics::DrawParam::default().dest([300.0, 300.0]),
        )?;
        graphics::draw(
            ctx,
            &graphics::Text::new(format!(
                "Rerequest Rate (%): {:.2}",
                self.delay_client.TEMP_rerequest_rate * 100.0
            )),
            graphics::DrawParam::default().dest([300.0, 350.0]),
        )?;
        graphics::present(ctx)
    }
}
