use crate::game::{GameInput, GameState};
use crate::rollback::RollbackClient;
use ggez::event::EventHandler;
use ggez::event::{KeyCode, KeyMods};
use ggez::{graphics, Context, GameResult};

pub struct RollbackRunner {
    current_state: Vec<GameState>,
    p1_input: Vec<GameInput>,
    p2_input: Vec<GameInput>,
    input_state: i32,
    player1: bool,
    client: RollbackClient,
}

impl RollbackRunner {
    pub fn new(ctx: &mut Context, player1: bool, client: RollbackClient) -> RollbackRunner {
        // Load/create resources such as images here.
        RollbackRunner {
            current_state: vec![GameState::new(ctx)],
            p1_input: Vec::new(),
            p2_input: Vec::new(),
            input_state: 0,
            player1,
            client,
        }
    }
}

impl EventHandler for RollbackRunner {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        if ggez::timer::check_update_time(ctx, 60) {
            let (target_player, other_player) = if self.player1 {
                (&mut self.p1_input, &mut self.p2_input)
            } else {
                (&mut self.p2_input, &mut self.p1_input)
            };

            target_player.insert(
                0,
                GameInput {
                    x_axis: self.input_state,
                },
            );
            other_player.insert(0, GameInput { x_axis: 0 });

            self.current_state[0].update(
                &self.p1_input[0..10.min(self.p1_input.len())],
                &self.p2_input[0..10.min(self.p2_input.len())],
            );
        }

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
