#![deny(warnings)]
use std::path::PathBuf;
use std::sync::Arc;
use std::net::SocketAddr;
use std::str::FromStr;
use std::vec::Vec;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use warp::Filter;
use warp::ws::WebSocket;

use netsketch_backend::*;



const NUM_ROOMS: usize = 100;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let mut rooms : Vec<Arc<Room>> = Vec::new();

    for i in 0..NUM_ROOMS{
        let mut room = Room::default();
        room.room_id = i;
        rooms.push(Arc::new(room));
    }

    let rooms = Arc::new(rooms);

    // Turn our "state" into a new Filter...
    let rooms = warp::any().map(move || rooms.clone());

    // GET /ws -> websocket upgrade
    let ws = warp::path("ws")
        .and(warp::path::param())
        .and(rooms)
        .and_then(|room_id: usize, rooms: Arc<Vec<Arc<Room>>>| async move{
            match rooms.get(room_id) {
                Some(x) => Ok(x.clone()),
                None => Err(warp::reject::not_found())
            } 
        })
        .and(warp::path::param())
        .and(warp::ws())
        .map(| room :Arc<Room>, username: String, ws: warp::ws::Ws|    {
            ws.on_upgrade(move |socket| connected(socket, room.clone(), username))
        });

    let args: Vec<String> = std::env::args().collect();

    let static_fs = warp::fs::dir(PathBuf::from(args[1].as_str()));

    warp::serve(ws.or(static_fs)).run(SocketAddr::from_str("[::]:8081").unwrap()).await;
    
    //let ipv4_warp = warp::serve(chat.clone()).try_bind(SocketAddr::from_str("0.0.0.0:8081").unwrap());
    //let ipv6_warp = warp::serve(chat.clone()).try_bind(SocketAddr::from_str("[::]:8081").unwrap());
    //futures::future::join(ipv6_warp, ipv4_warp).await;

}



async fn connected(ws: WebSocket, room: Arc<Room>, username: String) {


    // Split the socket into a sender and receive of messages.
    let (ws_tx, mut ws_rx) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));


    let userid = room.connect(tx, username).await;


    // Every time the user sends a message, broadcast it to
    // all other users...
    while let Some(result) = ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", userid, e);
                break;
            }
        };
        room.receive_msg(userid, msg).await;
    }


//    // ws_rx stream will keep processing as long as the user stays
//    // connected. Once they disconnect, then...
    room.disconnect(userid).await;

}

