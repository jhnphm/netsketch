use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use warp::ws::Message as WsMessage;
use tokio::sync::{mpsc, RwLock};
use std::vec::Vec;
use std::str;



pub struct User {
    pub name: String 
    // todo- password? 
}

pub struct Connection {
    pub user: User,
    pub sender: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>
}


pub struct Message {
    user: User,
    message: String
}

#[derive(Default)]
pub struct Room {
    pub connections: RwLock<HashMap<usize, Connection>>,
    chat_messages: RwLock<Vec<Message>>,
    canvas: RwLock<Vec<Layer>>
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





