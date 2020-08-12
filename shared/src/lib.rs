use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;
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

/// Positive signed integer specifying size of each side of square tile
pub const TILE_SIZE: i32 = 100;
/// Maximum number of layers supported
pub const MAX_LAYERS: u8 = 100;
/// Maximum levels of undo
pub const UNDO_SEARCH_DEPTH: usize = 100;

pub mod tile_ops {
    use crate::Offset;
    use crate::PaintStroke;
    use crate::TILE_SIZE;
    use std::collections::HashSet;

    /// Generates a tile offset containing the x and y coordinates specified
    pub fn point_to_tile_offset(x: i32, y: i32) -> Offset {
        let mut x = x;
        let mut y = y;
        if x < 0 {
            x -= TILE_SIZE;
        }
        if y < 0 {
            y -= TILE_SIZE;
        }
        Offset {
            x: (x / TILE_SIZE) * TILE_SIZE,
            y: (y / TILE_SIZE) * TILE_SIZE,
        }
    }
    /// Generates a hashset of tile offsets contained in rectangle specified by upper left and
    /// lower right offsets
    pub fn compute_bounded_tile_offsets(
        upper_left: &Offset,
        lower_right: &Offset,
    ) -> HashSet<Offset> {
        let upper_left_offset = point_to_tile_offset(upper_left.x, upper_left.y);
        let lower_right_offset = point_to_tile_offset(lower_right.x, lower_right.y);

        let num_x_tiles = (lower_right_offset.x - upper_left_offset.x) / TILE_SIZE + 1;
        let num_y_tiles = (lower_right_offset.y - upper_left_offset.y) / TILE_SIZE + 1;

        let mut offsets: HashSet<Offset> = HashSet::new();

        for i in 0..num_y_tiles {
            for j in 0..num_x_tiles {
                offsets.insert(Offset {
                    x: j * TILE_SIZE + upper_left_offset.x,
                    y: i * TILE_SIZE + upper_left_offset.y,
                });
            }
        }
        return offsets;
    }
    /// Finds tile offsets containing paint stroke
    pub fn find_paintstroke_tile_offsets(paint_stroke: &PaintStroke) -> HashSet<Offset> {
        // Use a hashset to deduplicate computed offsets
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
            // Loop through all paint stroke points in order to find tiles they belong to
            for i in &paint_stroke.points {
                let i = *i + *j;
                tile_offsets.insert(point_to_tile_offset(i.x, i.y));
            }
        }
        return tile_offsets;
    }
}

pub type StrokeIndex = usize;

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Layer {
    /// We store userid along w/ paint stroke for the purposes of per-user undo
    paint_strokes: Vec<PaintStroke>,
    /// Tile offsets are the upper left-most point
    /// Tiles contain list of indices in paint_strokes Vec belonging to that tile
    tiles: HashMap<Offset, Vec<StrokeIndex>>,
}

impl Layer {
    /// Adds paint stroke to layer and updates tile references, and consumes paint stroke. Returns
    /// paint stroke updated with order and hashset of updated tile offsets.
    pub fn add_paint_stroke(
        &mut self,
        user_id: UserId,
        mut paint_stroke: PaintStroke,
    ) -> (PaintStroke, HashSet<Offset>) {
        paint_stroke.user_id = user_id;

        // Note that stroke order can be different than index due to undoes, so we just add 1 to
        // the last order
        paint_stroke.order = if let Some(last_stroke) = self.paint_strokes.last() {
            last_stroke.order + 1
        } else {
            0
        };
        let tile_offsets = tile_ops::find_paintstroke_tile_offsets(&paint_stroke);
        self.paint_strokes.push(paint_stroke.clone());
        let stroke_index = self.paint_strokes.len() - 1;

        for i in &tile_offsets {
            if let Some(tile) = self.tiles.get_mut(&i) {
                tile.push(stroke_index);
            } else {
                let mut tile: Vec<StrokeIndex> = Vec::new();
                tile.push(stroke_index);
                self.tiles.insert(*i, tile);
            }
        }
        return (paint_stroke, tile_offsets);
    }
    /// Undoes actions done by specified user on paint stack. Returns hashset of updated tile
    /// offsets
    pub fn undo(&mut self, user_id: UserId) -> Option<HashSet<Offset>> {
        let start_idx = if UNDO_SEARCH_DEPTH <= self.paint_strokes.len() {
            self.paint_strokes.len() - UNDO_SEARCH_DEPTH
        } else {
            0
        };

        for (i, paint_stroke) in self.paint_strokes[start_idx..].iter().rev().enumerate() {
            if paint_stroke.user_id == user_id {
                let tile_offsets = tile_ops::find_paintstroke_tile_offsets(&paint_stroke);
                self.paint_strokes.remove(i);
                for j in &tile_offsets {
                    if let Some(tile) = self.tiles.get_mut(&j) {
                        for k in (0..tile.len()).rev() {
                            if tile[k] == i {
                                tile.remove(k);
                                break;
                            } else {
                                tile[k] = tile[k] - 1;
                            }
                        }
                    }
                }
                return Some(tile_offsets);
            }
        }
        return None;
    }

    /// Gets all strokes belonging to a tile
    pub fn get_tile_paintstrokes(&self, tile_offset: &Offset) -> BTreeSet<PaintStroke> {
        self.get_tile_paintstrokes_to_depth(tile_offset, usize::MAX)
    }
    /// Gets `max_depth` number of paintstrokes belonging to a tile
    pub fn get_tile_paintstrokes_to_depth(
        &self,
        tile_offset: &Offset,
        max_depth: usize,
    ) -> BTreeSet<PaintStroke> {
        if let Some(tile) = self.tiles.get(tile_offset) {
            let start_idx = if max_depth <= tile.len() {
                tile.len() - max_depth
            } else {
                0
            };
            tile[start_idx..]
                .iter()
                .map(|x| self.paint_strokes[*x].clone())
                .collect()
        } else {
            BTreeSet::new()
        }
    }
}

#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}

impl std::ops::Neg for Offset {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl std::ops::Add<Offset> for Offset {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Add<&Offset> for &Offset {
    type Output = Offset;

    fn add(self, rhs: &Offset) -> Offset {
        Offset {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
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

impl std::ops::Add<&Offset> for &StrokePoint {
    type Output = StrokePoint;

    fn add(self, rhs: &Offset) -> StrokePoint {
        StrokePoint {
            p: self.p,
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct PaintStroke {
    pub order: usize,
    pub user_id: UserId,
    pub brush: Brush,
    pub points: Vec<StrokePoint>,
}

impl PaintStroke {
    pub fn shift(&mut self, offset: &Offset) {
        self.points = self.points.iter().map(|x| x + offset).collect();
    }
}

impl Ord for PaintStroke {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order.cmp(&other.order)
    }
}

impl PartialOrd for PaintStroke {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PaintStroke {
    fn eq(&self, other: &Self) -> bool {
        self.order == other.order
    }
}

impl Eq for PaintStroke {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ClientMessage {
    PaintStroke(LayerId, PaintStroke),
    SetViewPort(Offset, Offset),
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
