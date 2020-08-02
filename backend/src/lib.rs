use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering}
};
use warp::ws::Message as WsMessage;
use tokio::sync::{mpsc, RwLock};
use std::vec::Vec;
use netsketch_shared as nss;


/// Our global unique user id counter.
static NEXT_USERID: AtomicUsize = AtomicUsize::new(1);


pub struct Connection {
    username: String,
    userid: nss::UserId,
    tx_conn: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>,
}


#[derive(Default)]
pub struct Room {
    pub room_id: usize,
    connections: RwLock<HashMap<nss::UserId, Connection>>,
    chat_messages: RwLock<Vec<(nss::UserName,nss::ChatMessage)>>,
    canvas: RwLock<Vec<nss::Layer>>
}


macro_rules! room_eprintln{
    ($room:ident,$($arg:tt)*) => {
        eprint!("Room ID: {}: ", $room.room_id);
        eprintln!($($arg)*);
    }
}

impl Room {


    pub async fn connect(&self, tx_conn: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>, username: String) -> nss::UserId{
        // Use a counter to assign a new unique ID for this user.
        let userid = NEXT_USERID.fetch_add(1, Ordering::Relaxed);

        let connection = Connection {username, userid, tx_conn};

        // Save the sender in our list of connected users.
        self.connections.write().await.insert(userid, connection);

        userid
    }

    pub async fn receive_msg(&self, user_id: nss::UserId, msg: WsMessage){
        use std::io::prelude::*;
        let mut buf = Vec::new();
        let mut inflator = flate2::read::DeflateDecoder::new(msg.as_bytes());
        if let Err(err) = inflator.read_to_end(&mut buf) {
            room_eprintln!(self, "Error decompressing input: {}", err.to_string());
            return;
        }
        let data: bincode::Result<nss::ClientMessage> = bincode::deserialize(&buf);
        let data =  match data 
        {
            Ok(data) => data,
            Err(err) => {room_eprintln!(self, "Error de-bincoding input: {}", err.to_string()); return;}
        };
        match data {
            nss::ClientMessage::PaintStroke(layer_id, paint_stroke) => {
                if layer_id < nss::MAX_LAYERS{
                    let mut canvas = self.canvas.write().await;
                    let layer = match canvas.get_mut(layer_id as usize){
                        Some(layer) => layer,
                        None=> {
                            canvas.resize(layer_id as usize+1, nss::Layer::default()); 
                            &mut canvas[layer_id as usize]
                        }
                    };
                    layer.add_paint_stroke(user_id, &paint_stroke);

                    for (their_user_id, conn) in self.connections.read().await.iter() {
                        if user_id != *their_user_id {
                            let msg = nss::ServerMessage::PaintStroke(layer_id, paint_stroke.clone());
                            //conn.tx_conn.send(Ok(msg));
                        }
                    }
                }else{
                    room_eprintln!(self,"Layer({}) > MAX_LAYERS", layer_id);
                }
            }
            _ => ()
        }

        // // Skip any non-Text messages...
        // let msg = if let Ok(s) = msg.to_str() {
        //     s
        // } else {
        //     return;
        // };

        // let new_msg = format!("<User#{}>: {}", my_id, msg);

        // // New message from this user, send it to everyone else (except same uid)...
        // for (&uid, tx) in users.read().await.iter() {
        //     if my_id != uid {
        //         if let Err(_disconnected) = tx.send(Ok(Message::text(new_msg.clone()))) {
        //             // The tx is disconnected, our `user_disconnected` code
        //             // should be happening in another task, nothing more to
        //             // do here.
        //         }
        //     }
        // }
    }

    pub async fn disconnect (&self, userid: nss::UserId){
        // Stream closed up, so remove from the user list
        let mut conn_map = self.connections.write().await;
        if let Some(conn) = conn_map.get(&userid){
            eprintln!("good bye user: {} {}", userid, conn.username);
            conn_map.remove(&userid);
        }

    }
}



