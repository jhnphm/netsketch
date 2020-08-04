use netsketch_shared::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::vec::Vec;
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message as WsMessage;

/// Our global unique user id counter.
static NEXT_USERID: AtomicUsize = AtomicUsize::new(1);

pub struct Connection {
    username: String,
    userid: UserId,
    tx_conn: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>,
}

#[derive(Default)]
pub struct Room {
    pub room_id: usize,
    connections: RwLock<HashMap<UserId, Connection>>,
    chat_messages: RwLock<Vec<(Username, ChatMessage)>>,
    canvas: RwLock<Vec<Layer>>,
}

macro_rules! room_eprintln{
    ($room:ident,$($arg:tt)*) => {
        eprint!("Room ID: {}: ", $room.room_id);
        eprintln!($($arg)*);
    }
}

impl Room {
    pub async fn connect(
        &self,
        tx_conn: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>,
        username: String,
    ) -> UserId {
        // Use a counter to assign a new unique ID for this user.
        let userid = NEXT_USERID.fetch_add(1, Ordering::Relaxed);

        let connection = Connection {
            username,
            userid,
            tx_conn,
        };

        // Save the sender in our list of connected users.
        self.connections.write().await.insert(userid, connection);

        userid
    }

    pub async fn receive_msg(&self, user_id: UserId, msg: WsMessage) {
        let dataresult: Result<ClientMessage, String> =
            netsketch_shared::from_zbincode(msg.as_bytes());

        let data = match dataresult {
            Ok(data) => data,
            Err(msg) => {
                room_eprintln!(self, "Deserialization error: {}", msg);
                return;
            }
        };
        match data {
            ClientMessage::PaintStroke(layer_id, paint_stroke) => {
                if layer_id < netsketch_shared::MAX_LAYERS {
                    let mut canvas = self.canvas.write().await;
                    let layer = match canvas.get_mut(layer_id as usize) {
                        Some(layer) => layer,
                        None => {
                            canvas.resize(layer_id as usize + 1, Layer::default());
                            &mut canvas[layer_id as usize]
                        }
                    };
                    layer.add_paint_stroke(user_id, &paint_stroke);

                    for (their_user_id, conn) in self.connections.read().await.iter() {
                        if user_id != *their_user_id {
                            let msg = ServerMessage::PaintStroke(layer_id, paint_stroke.clone());
                            //conn.tx_conn.send(Ok(msg));
                        }
                    }
                } else {
                    room_eprintln!(self, "Layer({}) > MAX_LAYERS", layer_id);
                }
            }
            _ => (),
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

    pub async fn disconnect(&self, userid: UserId) {
        // Stream closed up, so remove from the user list
        let mut conn_map = self.connections.write().await;
        if let Some(conn) = conn_map.get(&userid) {
            eprintln!("good bye user: {} {}", userid, conn.username);
            conn_map.remove(&userid);
        }
    }
}
