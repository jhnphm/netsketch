use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use warp::ws::Message as WsMessage;
use tokio::sync::{mpsc, RwLock};
use std::vec::Vec;
use std::str;
use netsketch_shared::*;


/// Our global unique user id counter.
static NEXT_USERID: AtomicUsize = AtomicUsize::new(1);


pub struct Connection {
    username: String,
    userid: UserId,
    tx_conn: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>,
}

pub struct Message {
    message: String
}

#[derive(Default)]
pub struct Room {
    connections: RwLock<HashMap<UserId, Connection>>,
    chat_messages: RwLock<Vec<(UserName,Message)>>,
    canvas: RwLock<Vec<Layer>>
}
impl Room {
    pub async fn connect(&self, tx_conn: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>, username: String) -> UserId{
        // Use a counter to assign a new unique ID for this user.
        let userid = NEXT_USERID.fetch_add(1, Ordering::Relaxed);

        eprintln!("New chat user: {} {}", userid, username);
        let connection = Connection {username, userid, tx_conn};

        // Save the sender in our list of connected users.
        self.connections.write().await.insert(userid, connection);

        userid
    }

    pub async fn receive_msg(&self, userid: UserId, msg: WsMessage){
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

    pub async fn disconnect (&self, userid: UserId){
        // Stream closed up, so remove from the user list
        let mut conn_map = self.connections.write().await;
        if let Some(conn) = conn_map.get(&userid){
            eprintln!("good bye user: {} {}", userid, conn.username);
            conn_map.remove(&userid);
        }

    }
}



