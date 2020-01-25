use crate::input_history::InputHistory;
use serde::{Deserialize, Serialize};

pub struct NetcodeClient<T> {
    local_player: InputHistory<T>,
    net_player: InputHistory<T>,
    current_frame: usize,
    held_input_count: usize,
    skip_frames: usize,
    pub ping: f32,
    pub TEMP_buffer_size: usize,
    pub TEMP_rerequest_rate: f32,
    pub TEMP_additional_input_delay: usize,
}

pub struct InputSet<'a, T> {
    pub local: &'a [T],
    pub net: &'a [T],
}

pub enum Action<'a, T> {
    DoNothing,
    Request(Packet<T>),
    RunInput(InputSet<'a, T>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Packet<T> {
    Inputs(usize, usize, Vec<T>),
    Request(usize),
    Provide(usize, Vec<T>),
}

impl<T: Clone + std::fmt::Debug + Default> NetcodeClient<T> {
    pub fn new(held_input_count: usize) -> Self {
        Self {
            local_player: InputHistory::new(),
            net_player: InputHistory::new(),
            current_frame: 0,
            held_input_count,
            ping: 0.0,
            skip_frames: 0,
            TEMP_buffer_size: 1,
            TEMP_rerequest_rate: 0.0,
            TEMP_additional_input_delay: 0,
        }
    }
    pub fn required_delay(&self) -> usize {
        ((self.ping + 3.0) / 32.0).ceil() as usize
    }
    pub fn input_delay(&self) -> usize {
        self.required_delay() + self.extra_input_delay()
    }
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    fn extra_input_delay(&self) -> usize {
        // was 3
        self.TEMP_additional_input_delay
    }
    fn delayed_current_frame(&self) -> usize {
        self.current_frame + self.input_delay()
    }

    //TODO turn this into an option
    pub fn handle_local_input(&mut self, data: T) -> Packet<T> {
        if !self.local_player.has_input(self.delayed_current_frame()) {
            // CORRECT THIS INPUT CHECKING
            // we want to send over teh most recent x inputs
            // and if we dont have enough inputs to send, tahts ok, but we dont wanna send them erroneously
            let target_input = self
                .local_player
                .add_local_input(self.delayed_current_frame(), data);
            let buffer_size = self.TEMP_buffer_size;
            let (frame, size) = if target_input.checked_sub(buffer_size - 1).is_some() {
                (target_input - (buffer_size - 1), buffer_size)
            } else {
                (target_input, 1)
            };
            Packet::Inputs(
                self.current_frame,
                frame,
                self.local_player
                    .get_inputs(target_input, size)
                    .iter()
                    .cloned()
                    .collect(),
            )
        } else {
            Packet::Inputs(self.current_frame, self.delayed_current_frame(), vec![])
        }
    }

    pub fn handle_net_input(&mut self, frame: usize, data: T) {
        if !self.net_player.has_input(frame) {
            self.net_player.add_network_input(frame, data);
        } else {
        }
    }

    pub fn handle_packet(&mut self, packet: Packet<T>) -> Option<Packet<T>> {
        match packet {
            Packet::Inputs(send_on_frame, start_frame, inputs) => {
                self.skip_frames = self
                    .current_frame
                    .checked_sub(send_on_frame + self.required_delay())
                    .unwrap_or(0);
                // todo fix sending multiple frames of data worth over
                for (idx, input) in inputs.into_iter().enumerate() {
                    //self.recieved_data.push((start_frame + idx, input));
                    self.handle_net_input(start_frame + idx, input);
                }
                None
            }
            Packet::Request(frame) => Some(Packet::Provide(
                frame,
                self.local_player
                    .get_inputs(frame, 1)
                    .iter()
                    .cloned()
                    .collect(),
            )),
            Packet::Provide(frame, inputs) => {
                for (idx, input) in inputs.into_iter().enumerate() {
                    self.net_player.add_network_input(frame + idx, input);
                }
                None
            }
        }
    }

    pub fn idle(&mut self) -> Action<T> {
        let TEMP_iir_duration = 600.0;
        if self.skip_frames > 0 {
            self.skip_frames -= 1;
            Action::DoNothing
        } else if self.local_player.has_input(self.current_frame)
            && self.net_player.has_input(self.current_frame)
        {
            let res = Action::RunInput(InputSet {
                local: self
                    .local_player
                    .get_inputs(self.current_frame, self.held_input_count),
                net: self
                    .net_player
                    .get_inputs(self.current_frame, self.held_input_count),
            });
            self.current_frame += 1;
            self.TEMP_rerequest_rate =
                self.TEMP_rerequest_rate * (TEMP_iir_duration - 1.0) / TEMP_iir_duration;
            res
        } else {
            self.TEMP_rerequest_rate = self.TEMP_rerequest_rate * (TEMP_iir_duration - 1.0)
                / TEMP_iir_duration
                + 1.0 / TEMP_iir_duration;
            Action::Request(Packet::Request(self.current_frame))
        }
    }
}
