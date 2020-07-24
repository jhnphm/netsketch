// #![deny(warnings)]
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use futures::{FutureExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use warp::ws::{Message, WebSocket};
use warp::Filter;
use std::str::FromStr;
use std::net::SocketAddr;
use std::vec::Vec;

use netsketch::*;


/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

const NUM_ROOMS: usize = 100;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let mut rooms : Vec<Arc<Room>> = Vec::new();

    for _ in 0..NUM_ROOMS{
        rooms.push(Arc::new(Room::default()));
    }

    let rooms = Arc::new(rooms);

    // Turn our "state" into a new Filter...
    let rooms = warp::any().map(move || rooms.clone());

    // GET /chat -> websocket upgrade
    let chat = warp::path("chat")
        .and(warp::path::param())
        .and(warp::path::param())
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(rooms)
        .map(|room_id: usize, username: String, ws: warp::ws::Ws, rooms: Arc<Vec<Arc<Room>>>| {
            let room =  match rooms.get(room_id) {
                Some(x) => Some(x.clone()),
                None => None
            };
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| connected(socket, room, username))
            
        });


    warp::serve(chat).run(SocketAddr::from_str("[::]:8081").unwrap()).await;
    
    //let ipv4_warp = warp::serve(chat.clone()).try_bind(SocketAddr::from_str("0.0.0.0:8081").unwrap());
    //let ipv6_warp = warp::serve(chat.clone()).try_bind(SocketAddr::from_str("[::]:8081").unwrap());
    //futures::future::join(ipv6_warp, ipv4_warp).await;

}

async fn fail_connected (){
}

async fn connected(ws: WebSocket, room: Option<Arc<Room>>, username: String) {
    if let Some(room) = room {

        // Use a counter to assign a new unique ID for this user.
        let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

        eprintln!("New chat user: {} {}", my_id, username);

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

        let connection = Connection {user: User {name: username}, sender: tx};

        // Save the sender in our list of connected users.
        room.connections.write().await.insert(my_id, connection);
//
//    // Return a `Future` that is basically a state machine managing
//    // this specific user's connection.
//
//    // Make an extra clone to give to our disconnection handler...
//    let users2 = users.clone();
//
//    // Every time the user sends a message, broadcast it to
//    // all other users...
//    while let Some(result) = ws_rx.next().await {
//        let msg = match result {
//            Ok(msg) => msg,
//            Err(e) => {
//                eprintln!("websocket error(uid={}): {}", my_id, e);
//                break;
//            }
//        };
//        user_message(my_id, msg, &users).await;
//    }
//
//    // user_ws_rx stream will keep processing as long as the user stays
//    // connected. Once they disconnect, then...
//    user_disconnected(my_id, &users2).await;

    }
}

//async fn user_message(my_id: usize, msg: Message, users: &Users) {
//    // Skip any non-Text messages...
//    let msg = if let Ok(s) = msg.to_str() {
//        s
//    } else {
//        return;
//    };
//
//    let new_msg = format!("<User#{}>: {}", my_id, msg);
//
//    // New message from this user, send it to everyone else (except same uid)...
//    for (&uid, tx) in users.read().await.iter() {
//        if my_id != uid {
//            if let Err(_disconnected) = tx.send(Ok(Message::text(new_msg.clone()))) {
//                // The tx is disconnected, our `user_disconnected` code
//                // should be happening in another task, nothing more to
//                // do here.
//            }
//        }
//    }
//}
//
//async fn user_disconnected(my_id: usize, users: &Users) {
//    eprintln!("good bye user: {}", my_id);
//
//    // Stream closed up, so remove from the user list
//    users.write().await.remove(&my_id);
//}
//
