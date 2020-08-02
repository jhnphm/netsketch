use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub type LayerId = u8;
pub type UserId = usize;
pub type UserName = String;
pub type ChatMessage = String;

pub const TILE_SIZE: u32 = 1024;
pub const MAX_LAYERS: u8 = 100;


#[derive(Default, Debug, PartialEq, Clone)]
pub struct Layer {
    paint_strokes: Vec<(UserId, PaintStroke)>,
    tiles: HashMap<Offset, Tile>,
}

impl Layer{
    pub fn add_paint_stroke(&mut self, user_id: UserId, paint_stroke: &PaintStroke){
        self.paint_strokes.push((user_id, paint_stroke.clone()));
        // TODO Add to tiles
    }
}


#[derive(Default, Debug, PartialEq, Clone)]
pub struct Tile {
    stroke_indices: Vec<usize>,
}

#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}


#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Brush {
    pub color: Color,
    pub width: f32,
    pub hardness: f32,
    pub smudging: f32,
}

impl Default for Brush {
    fn default() -> Self {
        Brush {
            color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            width: 1.0,
            hardness: 1.0,
            smudging: 1.0,
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct StrokePoint {
    /// Pressure
    pub p: f32,
    /// X coord
    pub x: i32,
    /// Y coord
    pub y: i32,
}

impl std::ops::Add<Offset> for StrokePoint {
    type Output = StrokePoint;

    fn add(self, rhs: Offset) -> StrokePoint {
        StrokePoint {
            p: self.p,
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PaintStroke {
    pub brush: Brush,
    pub points: Vec<StrokePoint>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ClientMessage {
    PaintStroke(LayerId, PaintStroke),
    ChatMessage(String),
    UndoMessage,
    FetchTile(LayerId, Offset),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ServerMessage {
    PaintStroke(LayerId, PaintStroke),
    ChatMessage(UserName, String),
}

//pub fn to_zbincode<T: Serialize>
