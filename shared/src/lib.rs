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

pub struct Layer {
    paint_strokes: Vec<(UserId,PaintStroke)>,
    tiles: HashMap<Coord, Tile>
}


struct Coord{
    x: i32,
    y: i32
}

const TILE_SIZE: u32 = 1024;

struct Tile {
    coord: Coord,
    stroke_indices: Vec<usize>
}


struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}

struct Brush{
    color: Color,
    width: f32,
    hardness: f32,
    smudging: f32
}


struct StrokePoint {
    pressure: f32,
    x: i32,
    y: i32
}

struct PaintStroke{
    brush: Brush,
    points: Vec<StrokePoint>
}
