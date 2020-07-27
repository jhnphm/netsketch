use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub type UserId = usize;
pub type UserName = String;

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Layer {
    paint_strokes: Vec<(UserId, PaintStroke)>,
    tiles: HashMap<Offset, Tile>,
}

#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
struct Offset {
    x: i32,
    y: i32,
}

const TILE_SIZE: u32 = 1024;

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Clone)]
struct Tile {
    stroke_indices: Vec<usize>,
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
