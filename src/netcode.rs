use crate::input_history::{LocalHistory, NetworkedHistory, PredictionResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

// TODO, consider parameterizing the size of current_frame to not waste bytes on the fact that its
// at least 4 bytes when 18 minutes of 60 FPS gameplay only needs a u16 (2 bytes)
// TODO, add a bunch of functions to perform syncing of the clients, but not pass input back and forth
// TODO, make everything return a Result, taht way users can decide whether or not to handle errorsalso

// TODO, create getters/setters for all the public properties

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
enum PlayerType {
    Local,
    Net,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PlayerHandle(usize);

impl PlayerHandle {
    pub fn id(&self) -> usize {
        self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct PlayerInfo {
    player_type: PlayerType,
    id: PlayerHandle,
}

pub struct InputSet<'a, Input> {
    pub inputs: Vec<&'a [Input]>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Packet<Input> {
    Inputs(PlayerHandle, usize, usize, Vec<Input>),
    Request(usize),
    Provide(Vec<(PlayerHandle, usize, Vec<Input>)>),
}
pub struct NetcodeClient<Input, GameState> {
    local_players: HashMap<PlayerHandle, LocalHistory<Input>>,
    net_players: HashMap<PlayerHandle, NetworkedHistory<Input>>,
    current_frame: usize,
    held_input_count: usize,
    skip_frames: usize,
    saved_rollback_states: HashMap<usize, GameState>,
    rollback_to: Option<(usize, GameState)>,
    players: Vec<PlayerInfo>,
    network_delay: HashMap<PlayerHandle, usize>,
    input_delay: usize,
    allowed_rollback: usize,
    packet_buffer_size: usize,
}

impl<Input: Clone + Default + PartialEq + std::fmt::Debug, GameState: std::fmt::Debug>
    NetcodeClient<Input, GameState>
{
    pub fn new(held_input_count: usize) -> Self {
        Self {
            local_players: HashMap::new(),
            net_players: HashMap::new(),
            current_frame: 0,
            held_input_count,
            skip_frames: 0,
            packet_buffer_size: 10,
            input_delay: 1,
            network_delay: HashMap::new(),
            saved_rollback_states: HashMap::new(),
            allowed_rollback: 9,
            rollback_to: None,
            players: Vec::new(),
        }
    }

    pub fn input_delay(&self) -> usize {
        self.input_delay
    }
    pub fn set_input_delay(&mut self, value: usize) {
        self.input_delay = value;
    }
    pub fn allowed_rollback(&self) -> usize {
        self.allowed_rollback
    }
    pub fn set_allowed_rollback(&mut self, value: usize) {
        self.allowed_rollback = value;
    }
    pub fn packet_buffer_size(&self) -> usize {
        self.packet_buffer_size
    }
    pub fn set_packet_buffer_size(&mut self, value: usize) {
        self.packet_buffer_size = value;
    }

    pub fn get_network_delay(&self, player: PlayerHandle) -> usize {
        assert!(
            self.players[player.id()].player_type == PlayerType::Net,
            "Must handle networked input for a networked player."
        );

        self.network_delay[&player]
    }
    pub fn set_network_delay(&mut self, value: usize, player: PlayerHandle) {
        assert!(
            self.players[player.id()].player_type == PlayerType::Net,
            "Must handle networked input for a networked player."
        );

        self.network_delay.insert(player, value);
    }

    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    fn delayed_current_frame(&self) -> usize {
        self.current_frame + self.input_delay
    }

    pub fn add_local_player(&mut self, index: usize) -> PlayerHandle {
        let handle = PlayerHandle(index);
        let info: PlayerInfo = PlayerInfo {
            id: handle,
            player_type: PlayerType::Local,
        };
        self.local_players.insert(handle, LocalHistory::new());
        self.players.push(info);
        self.players
            .sort_by(|lhs, rhs| lhs.id.id().cmp(&rhs.id.id()));

        handle
    }
    pub fn add_net_player(&mut self, index: usize) -> PlayerHandle {
        let handle = PlayerHandle(index);
        let info: PlayerInfo = PlayerInfo {
            id: handle,
            player_type: PlayerType::Net,
        };
        self.net_players.insert(handle, NetworkedHistory::new());
        self.players.push(info);
        self.players
            .sort_by(|lhs, rhs| lhs.id.id().cmp(&rhs.id.id()));
        self.network_delay.insert(handle, 0);

        handle
    }

    pub fn handle_local_input(
        &mut self,
        data: Input,
        player: PlayerHandle,
    ) -> Option<Packet<Input>> {
        assert!(
            self.players[player.id()].player_type == PlayerType::Local,
            "Must add local input to a local player."
        );
        let delayed_current_frame = self.delayed_current_frame();
        let local_player = self.local_players.get_mut(&player).unwrap();
        if !local_player.has_input(delayed_current_frame) {
            // CORRECInput InputHIS INPUInput CHECKING
            // we want to send over teh most recent x inputs
            // and if we dont have enough inputs to send, tahts ok, but we dont wanna send them erroneously
            let input_frame = local_player.add_input(data);
            let buffer_size = self.packet_buffer_size;

            let (range, data) = local_player.get_inputs(input_frame, buffer_size);

            Some(Packet::Inputs(
                player,
                self.current_frame,
                range.first,
                data.iter().cloned().collect(),
            ))
        } else {
            None
        }
    }

    pub fn handle_net_input(&mut self, frame: usize, input: Input, player: PlayerHandle) {
        assert!(
            self.players[player.id()].player_type == PlayerType::Net,
            "Must handle networked input for a networked player."
        );

        let net_player = self.net_players.get_mut(&player).unwrap();
        match net_player.add_input(frame, input) {
            PredictionResult::Unpredicted => (),
            PredictionResult::Correct => {
                if self
                    .net_players
                    .iter()
                    .all(|(_, net_player)| !net_player.is_predicted_input(frame))
                {
                    let removed_state = self.saved_rollback_states.remove(&frame);
                    assert!(
                        removed_state.is_some(),
                        "Correct prediction with no remaining predictions for that frame should have a corresponding save state to drop."
                    );
                }
            }
            PredictionResult::Wrong => {
                let state = self.saved_rollback_states.remove(&frame);
                // TODO: this assert might not be true anymore if multiple people have mispredicted and the state has been removed for oen of them already
                // instead we should verify that either, there is one to be rollback'd to, or that we're already rollbacking to this mispredicteds
                // input, or earlier

                // prefer to rollback to an older frame if one's available
                if let Some(state) = state {
                    if let Some((old_frame, _)) = self.rollback_to {
                        if old_frame > frame {
                            // we're rollbacking to an older frame so we can replace it
                            self.rollback_to = Some((frame, state));
                        } else {
                            // should be ok to lose this state, because the rollback will recreate it if necessary
                            // or we're already rolling back to the correct place
                        }
                    } else {
                        // we're not already rolling back, so lets go
                        self.rollback_to = Some((frame, state));
                    }
                } else if self.rollback_to.is_none() {
                    // panic if we don't have a state to rollback to
                    panic!("No state to rollback to.");
                }
            }
        }
    }

    // must return to sender
    pub fn handle_packet(&mut self, packet: Packet<Input>) -> Option<Packet<Input>> {
        match packet {
            Packet::Inputs(player_handle, sent_on_frame, start_frame, inputs) => {
                assert!(
                    self.players[player_handle.0].player_type == PlayerType::Net,
                    "Must handle networked input for a networked player."
                );

                self.skip_frames = self
                    .current_frame
                    .checked_sub(sent_on_frame + self.get_network_delay(player_handle))
                    .unwrap_or(0);
                for (idx, input) in inputs.into_iter().enumerate() {
                    let frame = start_frame + idx;
                    self.handle_net_input(frame, input, player_handle);
                }
                None
            }
            Packet::Request(frame) => {
                // TODO iterate through all local players and send out multiple sets of data, one for each local player

                let requested_data: Vec<_> = self
                    .local_players
                    .iter()
                    .map(|(handle, player)| (handle, player.get_inputs(frame, 1)))
                    .map(|(handle, (range, player))| {
                        (*handle, range.first, player.iter().cloned().collect())
                    })
                    .collect();
                if requested_data.is_empty() {
                    // we don't have any local players or anything to send back.
                    None
                } else {
                    Some(Packet::Provide(requested_data))
                }
            }
            Packet::Provide(inputs_list) => {
                for (player_handle, frame, inputs) in inputs_list {
                    for (idx, input) in inputs.into_iter().enumerate() {
                        self.handle_net_input(frame + idx, input, player_handle);
                    }
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
                    !self
                        .net_players
                        .iter()
                        .any(|(_, net_player)| net_player.is_empty_input(rollback_current_frame)),
                    "Can't rollback through empty data."
                );

                if self
                    .net_players
                    .iter()
                    .any(|(_, net_player)| net_player.is_predicted_input(rollback_current_frame))
                {
                    self.saved_rollback_states
                        .insert(rollback_current_frame, game.save_state());
                }
                for (_, net_player) in self
                    .net_players
                    .iter_mut()
                    .filter(|(_, net_player)| net_player.is_predicted_input(rollback_current_frame))
                {
                    net_player.repredict(rollback_current_frame);
                }

                game.advance_frame(InputSet {
                    inputs: self
                        .players
                        .iter()
                        .map(|info| {
                            //
                            let (range, inputs) = match info.player_type {
                                PlayerType::Local => self.local_players[&info.id]
                                    .get_inputs(rollback_current_frame, self.held_input_count),
                                PlayerType::Net => self.net_players[&info.id]
                                    .get_inputs(rollback_current_frame, self.held_input_count),
                            };
                            assert!(range.last == rollback_current_frame, "The last frame of input in the queue, should match the currently rollbacking frame.");
                            inputs
                        })
                        .collect(),
                });
            }
        }

        if self.skip_frames > 0 {
            self.skip_frames -= 1;
            None
        } else if self
            .local_players
            .iter()
            .all(|(_, local_player)| local_player.has_input(self.current_frame))
            && self
                .net_players
                .iter()
                .all(|(_, net_players)| net_players.has_input(self.current_frame))
        {
            if self.current_frame % self.held_input_count == 0 {
                let clear_target = self
                    .current_frame
                    .checked_sub(self.held_input_count + self.allowed_rollback)
                    .unwrap_or(0);
                for (_, local_player) in self.local_players.iter_mut() {
                    local_player.clean(clear_target);
                }
                for (_, net_player) in self.net_players.iter_mut() {
                    net_player.clean(clear_target);
                }
            }

            game.advance_frame(InputSet {
                inputs: self
                    .players
                    .iter()
                    .map(|info| {
                        //
                        let (range, inputs) = match info.player_type {
                            PlayerType::Local => self.local_players[&info.id]
                                .get_inputs(self.current_frame, self.held_input_count),
                            PlayerType::Net => self.net_players[&info.id]
                                .get_inputs(self.current_frame, self.held_input_count),
                        };
                        assert!(range.last == self.current_frame, "The last frame of input in the queue, should match the currently rollbacking frame.");
                        inputs
                    })
                    .collect(),
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
                && self.current_frame > self.allowed_rollback
            {
                self.saved_rollback_states
                    .insert(self.current_frame, game.save_state());

                let current_frame = self.current_frame;

                for net_player in self
                    .net_players
                    .values_mut()
                    .filter(|net_player| net_player.is_empty_input(current_frame))
                {
                    net_player.predict(self.current_frame);
                }

                game.advance_frame(InputSet {
                    inputs: self
                        .players
                        .iter()
                        .map(|info| {
                            //
                            let (range, inputs) = match info.player_type {
                                PlayerType::Local => self.local_players[&info.id]
                                    .get_inputs(self.current_frame, self.held_input_count),
                                PlayerType::Net => self.net_players[&info.id]
                                    .get_inputs(self.current_frame, self.held_input_count),
                            };
                            assert!(range.last == self.current_frame, "The last frame of input in the queue, should match the currently rollbacking frame.");
                            inputs
                        })
                        .collect(),
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
