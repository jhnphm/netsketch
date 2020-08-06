use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;

pub mod prelude;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub type LayerId = u8;
pub type UserId = usize;
pub type Username = String;
pub type ChatMessage = String;

pub const TILE_SIZE: u32 = 100;
pub const MAX_LAYERS: u8 = 100;
pub const UNDO_DEPTH: u8 = 10;

type StrokeIndex = usize;

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Layer {
    paint_strokes: Vec<(UserId, PaintStroke)>,
    tiles: HashMap<Offset, Vec<StrokeIndex>>,
}

impl Layer {
    fn find_tile_offsets(paint_stroke: &PaintStroke) -> HashSet<Offset> {
        let mut tile_offsets: HashSet<Offset> = HashSet::new();

        // We have to add offsets in each direction to account for brush radius
        let radius = ((paint_stroke.brush.width + 1.0) / 2.0) as i32;

        for j in &[
            Offset {
                x: -radius,
                y: -radius,
            },
            Offset {
                x: -radius,
                y: radius,
            },
            Offset {
                x: radius,
                y: -radius,
            },
            Offset {
                x: radius,
                y: radius,
            },
        ] {
            for i in &paint_stroke.points {
                let i = *i + *j;
                let mut x = i.x;
                let mut y = i.y;
                if x < 0 {
                    x -= TILE_SIZE as i32;
                }
                if y < 0 {
                    y -= TILE_SIZE as i32;
                }
                let offset = Offset {
                    x: (x / TILE_SIZE as i32) * TILE_SIZE as i32,
                    y: (y / TILE_SIZE as i32) * TILE_SIZE as i32,
                };
                tile_offsets.insert(offset);
            }
        }
        return tile_offsets;
    }
    pub fn add_paint_stroke(&mut self, user_id: UserId, paint_stroke: &PaintStroke) {
        self.paint_strokes.push((user_id, paint_stroke.clone()));
        let stroke_index = self.paint_strokes.len() - 1;

        let tile_offsets = Layer::find_tile_offsets(paint_stroke);

        for i in tile_offsets {
            if let Some(tile) = self.tiles.get_mut(&i) {
                tile.push(stroke_index);
            } else {
                let mut tile: Vec<StrokeIndex> = Vec::new();
                tile.push(stroke_index);
                self.tiles.insert(i, tile);
            }
        }
    }

}

#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
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

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
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
    ChatMessage(Username, String),
}

pub fn from_zbincode<T: serde::de::DeserializeOwned>(serialized: &[u8]) -> Result<T, String> {
    use std::io::prelude::*;
    let mut buf = Vec::new();
    let mut inflator = flate2::read::DeflateDecoder::new(serialized);
    if let Err(err) = inflator.read_to_end(&mut buf) {
        return Err(err.to_string());
    }
    let data: bincode::Result<T> = bincode::deserialize(&buf);

    match data {
        Ok(data) => Ok(data),
        Err(err) => Err(err.to_string()),
    }
}
pub fn to_zbincode<T: Serialize>(serializable: &T) -> Result<Vec<u8>, String> {
    let bincode = match bincode::serialize(&serializable) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };

    use std::io::Write;
    let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    if let Err(err) = e.write_all(&bincode) {
        return Err(err.to_string());
    }

    match e.finish() {
        Ok(data) => Ok(data),
        Err(err) => Err(err.to_string()),
    }
}
