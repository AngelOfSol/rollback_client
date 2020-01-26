use crate::input_history::{LocalHistory, NetworkedHistory, PredictionResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// TODO, consider parameterizing the size of current_frame to not waste bytes on the fact that its
// at least 4 bytes when 18 minutes of 60 FPS gameplay only needs a u16 (2 bytes)
// TODO, add a bunch of functions to perform syncing of the clients, but not pass input back and forth

pub struct NetcodeClient<Input, GameState> {
    local_player: LocalHistory<Input>,
    net_player: NetworkedHistory<Input>,
    current_frame: usize,
    held_input_count: usize,
    skip_frames: usize,
    saved_rollback_states: HashMap<usize, GameState>,
    rollback_to: Option<(usize, GameState)>,

    pub network_delay: usize,
    pub input_delay: usize,
    pub allowed_rollback: usize,
    pub packet_buffer_size: usize,
}

pub struct InputSet<'a, Input> {
    pub local: &'a [Input],
    pub net: &'a [Input],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Packet<Input> {
    Inputs(usize, usize, Vec<Input>),
    Request(usize),
    Provide(usize, Vec<Input>),
}

impl<Input: Clone + Default + PartialEq + std::fmt::Debug, GameState: std::fmt::Debug>
    NetcodeClient<Input, GameState>
{
    pub fn new(held_input_count: usize) -> Self {
        Self {
            local_player: LocalHistory::new(),
            net_player: NetworkedHistory::new(),
            current_frame: 0,
            held_input_count,
            skip_frames: 0,
            packet_buffer_size: 10,
            input_delay: 1,
            network_delay: 0,
            saved_rollback_states: HashMap::new(),
            allowed_rollback: 9,
            rollback_to: None,
        }
    }
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    fn delayed_current_frame(&self) -> usize {
        self.current_frame + self.input_delay
    }

    //InputODO turn this into an option
    pub fn handle_local_input(&mut self, data: Input) -> Option<Packet<Input>> {
        if !self.local_player.has_input(self.delayed_current_frame()) {
            // CORRECInput InputHIS INPUInput CHECKING
            // we want to send over teh most recent x inputs
            // and if we dont have enough inputs to send, tahts ok, but we dont wanna send them erroneously
            let input_frame = self.local_player.add_input(data);
            let buffer_size = self.packet_buffer_size;

            let (range, data) = self.local_player.get_inputs(input_frame, buffer_size);

            Some(Packet::Inputs(
                self.current_frame,
                range.first,
                data.iter().cloned().collect(),
            ))
        } else {
            None
        }
    }

    pub fn handle_net_input(&mut self, frame: usize, input: Input) {
        match self.net_player.add_input(frame, input) {
            PredictionResult::Unpredicted => (),
            PredictionResult::Correct => {
                let removed_state = self.saved_rollback_states.remove(&frame);
                assert!(
                    removed_state.is_some(),
                    "Correct prediction should havea corresponding save state to drop."
                );
            }
            PredictionResult::Wrong => {
                dbg!(self.current_frame());
                dbg!(frame);
                let state = self.saved_rollback_states.remove(&frame);
                dbg!(&self.saved_rollback_states.keys().collect::<Vec<_>>());
                assert!(
                    state.is_some(),
                    "Misprediction should have a corrseponding save state."
                );
                if let Some((old_frame, _)) = self.rollback_to {
                    if old_frame > frame {
                        self.rollback_to = Some((frame, state.unwrap()));
                    } else {
                    }
                } else {
                    self.rollback_to = Some((frame, state.unwrap()));
                }
            }
        }
    }

    pub fn handle_packet(&mut self, packet: Packet<Input>) -> Option<Packet<Input>> {
        match packet {
            Packet::Inputs(sent_on_frame, start_frame, inputs) => {
                self.skip_frames = self
                    .current_frame
                    .checked_sub(sent_on_frame + self.network_delay)
                    .unwrap_or(0);
                for (idx, input) in inputs.into_iter().enumerate() {
                    let frame = start_frame + idx;
                    self.handle_net_input(frame, input);
                }
                None
            }
            Packet::Request(frame) => {
                let (range, data) = self.local_player.get_inputs(frame, 1);

                Some(Packet::Provide(range.first, data.iter().cloned().collect()))
            }
            Packet::Provide(frame, inputs) => {
                for (idx, input) in inputs.into_iter().enumerate() {
                    self.handle_net_input(frame + idx, input);
                }
                None
            }
        }
    }

    pub fn update<'a, Game: RollbackableGameState<SavedState = GameState, Input = Input>>(
        &'a mut self,
        game: &mut Game,
    ) -> Option<Packet<Input>> {
        if let Some((rollback_frame, state)) = self.rollback_to.take() {
            game.load_state(state);

            for rollback_current_frame in rollback_frame..self.current_frame {
                assert!(
                    !self.net_player.is_empty_input(rollback_current_frame),
                    "Can't rollback through empty data."
                );

                if self.net_player.is_predicted_input(rollback_current_frame) {
                    self.saved_rollback_states
                        .insert(rollback_current_frame, game.save_state());
                    self.net_player.repredict(rollback_current_frame);
                }

                let (range, net_player_inputs) = self
                    .net_player
                    .get_inputs(rollback_current_frame, self.held_input_count);

                assert_eq!(range.last, rollback_current_frame, "The last frame of input in the queue, should match the currently rollbacking frame.");

                game.advance_frame(InputSet {
                    local: self
                        .local_player
                        .get_inputs(rollback_current_frame, self.held_input_count)
                        .1,
                    net: net_player_inputs,
                });
            }
        }

        if self.skip_frames > 0 {
            self.skip_frames -= 1;
            None
        } else if self.local_player.has_input(self.current_frame)
            && self.net_player.has_input(self.current_frame)
        {
            if self.current_frame % self.held_input_count == 0 {
                let clear_target = self
                    .current_frame
                    .checked_sub(self.held_input_count + self.allowed_rollback)
                    .unwrap_or(0);
                self.local_player.clean(clear_target);
                self.net_player.clean(clear_target);
            }

            game.advance_frame(InputSet {
                local: self
                    .local_player
                    .get_inputs(self.current_frame, self.held_input_count)
                    .1,
                net: self
                    .net_player
                    .get_inputs(self.current_frame, self.held_input_count)
                    .1,
            });

            self.current_frame += 1;

            None
        } else {
            if self
                .saved_rollback_states
                .keys()
                .min()
                .and_then(|frame| self.current_frame.checked_sub(*frame))
                .unwrap_or(0)
                < self.allowed_rollback
                && self.current_frame > self.allowed_rollback * 60
            {
                self.saved_rollback_states
                    .insert(self.current_frame, game.save_state());

                self.net_player.predict(self.current_frame);

                game.advance_frame(InputSet {
                    local: self
                        .local_player
                        .get_inputs(self.current_frame, self.held_input_count)
                        .1,
                    net: self
                        .net_player
                        .get_inputs(self.current_frame, self.held_input_count)
                        .1,
                });
                self.current_frame += 1;

                None
            } else {
                Some(Packet::Request(self.current_frame))
            }
        }
    }
}

pub trait RollbackableGameState {
    type Input;
    type SavedState;
    fn advance_frame(&mut self, input: InputSet<'_, Self::Input>);
    fn save_state(&self) -> Self::SavedState;
    fn load_state(&mut self, load: Self::SavedState);
}
