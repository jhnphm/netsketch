use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{mpsc, RwLock};
use std::vec::Vec;
use std::str;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);


struct User {
    name: String 
    // todo- password? 
}

struct Connection {
    user: User,
    sender: mpsc::UnboundedSender<Result<Message, warp::Error>>
}


struct Message {
    user: User,
    message: String
}

struct Room {
    connections: RwLock<HashMap<usize, Connection>>,
    chat_messages: RwLock<Vec<Message>>,
    canvas: RwLock<Canvas>
}


struct Canvas{
    layers: RwLock<Vec<Layer>>
}

struct Layer {
    paint_strokes: Vec<PaintStroke>,
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
    userid: usize,
    brush: Brush,
    points: Vec<StrokePoint>
}

