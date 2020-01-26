use ggez::graphics::{self, Image};
use ggez::{self, Context, GameResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Player {
    x: i32,
    graphic: Image,
}

#[derive(Clone, Debug)]
pub struct GameState {
    p1: Player,
    p2: Player,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GameInput {
    pub x_axis: i32,
}

impl Default for GameInput {
    fn default() -> Self {
        Self { x_axis: 0 }
    }
}

pub type PlayerInputHistory = GameInput;

impl GameState {
    pub fn new(ctx: &mut Context) -> Self {
        Self {
            p1: Player {
                x: -100,
                graphic: Image::new(ctx, "/imgs/p1.png").unwrap(),
            },
            p2: Player {
                x: 100,
                graphic: Image::new(ctx, "/imgs/p2.png").unwrap(),
            },
        }
    }

    pub fn update(&mut self, p1: &PlayerInputHistory, p2: &PlayerInputHistory) {
        if p1.x_axis > 0 {
            self.p1.x += 4;
        } else if p1.x_axis < 0 {
            self.p1.x -= 4;
        }

        if p2.x_axis > 0 {
            self.p2.x += 4;
        } else if p2.x_axis < 0 {
            self.p2.x -= 4;
        }
    }

    pub fn draw(&self, ctx: &mut Context, y_offset: f32) -> GameResult<()> {
        graphics::draw(
            ctx,
            &self.p1.graphic,
            graphics::DrawParam::default().dest([self.p1.x as f32 + 400.0, y_offset]),
        )?;
        graphics::draw(
            ctx,
            &self.p2.graphic,
            graphics::DrawParam::default().dest([self.p2.x as f32 + 400.0, y_offset]),
        )
    }
}
