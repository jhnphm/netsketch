use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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


pub fn from_zbincode<T: serde::de::DeserializeOwned>(serialized: &[u8])->Result<T, String>{
    use std::io::prelude::*;
    let mut buf = Vec::new();
    let mut inflator = flate2::read::DeflateDecoder::new(serialized);
    if let Err(err) = inflator.read_to_end(&mut buf) {
        return Err(err.to_string());
    }
    let data: bincode::Result<T> = bincode::deserialize(&buf);

    match data 
    {
        Ok(data) => Ok(data),
        Err(err) => Err(err.to_string())
    }
}
pub fn to_zbincode<T: Serialize>(serializable: &T)->Result<Vec<u8>, String>{
    let bincode = match bincode::serialize(&serializable){
        Ok(data) => data,
        Err(err) => return Err(err.to_string())
    };
    
    use std::io::Write;
    let mut e =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    if let Err(err) = e.write_all(&bincode){
        return Err(err.to_string());
    }

    match  e.finish(){
        Ok(data) => Ok(data),
        Err(err) => Err(err.to_string())
    }
}


