use netsketch_shared::prelude::*;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::vec::Vec;
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message as WsMessage;

/// Our global unique user id counter.
static NEXT_USERID: AtomicUsize = AtomicUsize::new(1);

pub struct Connection {
    username: String,
    //userid: UserId,
    tx_conn: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>,
    active_tile_offsets: HashSet<Offset>,
}

#[derive(Default)]
pub struct Room {
    pub room_id: usize,
    connections: RwLock<HashMap<UserId, Connection>>,
    //chat_messages: RwLock<Vec<(Username, ChatMessage)>>,
    canvas: RwLock<Vec<ServerLayer>>,
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
            tx_conn,
            active_tile_offsets: HashSet::default(),
        };

        // Save the sender in our list of connected users.
        self.connections.write().await.insert(userid, connection);

        userid
    }

    pub async fn receive_msg(&self, user_id: UserId, msg: WsMessage) {
        if msg.is_binary() {
            // Deserialize from compressed bincode
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
                // Paintstroke received
                ClientMessage::PaintStroke(layer_id, mut paint_stroke) => {
                    // Bounds check on layer IDs
                    if layer_id < netsketch_shared::MAX_LAYERS {
                        let mut canvas = self.canvas.write().await;

                        // If nonexistant layer, create it and everything in between
                        let layer = match canvas.get_mut(layer_id as usize) {
                            Some(layer) => layer,
                            None => {
                                canvas.resize(layer_id as usize + 1, ServerLayer::default());
                                &mut canvas[layer_id as usize]
                            }
                        };

                        paint_stroke.user_id = user_id;

                        // Add stroke to paint stack
                        let (paint_stroke, tile_offsets) =
                            layer.add_paint_stroke(paint_stroke);

                        // Send paint stroke to everyone connected viewing the visible tiles
                        let msg = if user_id != *their_user_id {
                            ServerMessage::PaintStroke(layer_id, (*paint_stroke).clone());
                        }else{
                            ServerMessage::PaintStrokeEcho(layer_id, (*paint_stroke).clone());
                        }

                        let zbincode_msg = netsketch_shared::to_zbincode(&msg);

                        match zbincode_msg {
                            Ok(msg) => {
                                for (their_user_id, conn) in self.connections.read().await.iter() {
                                    if  tile_offsets
                                            .intersection(&conn.active_tile_offsets)
                                            .count()
                                            != 0
                                    {
                                        if let Err(err) =
                                            conn.tx_conn.send(Ok(WsMessage::binary(msg.clone())))
                                        {
                                            room_eprintln!(self, "Send error: {}", err.to_string());
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                room_eprintln!(self, "ZBincode error: {}", err.to_string());
                            }
                        };
                    } else {
                        // Bail out on failed bounds check
                        room_eprintln!(self, "Layer({}) > MAX_LAYERS", layer_id);
                    }
                }
                ClientMessage::SetViewPort(upper_left, lower_right) => {
                    if let Some(conn) = self.connections.write().await.get_mut(&user_id) {
                        conn.active_tile_offsets =
                            netsketch_shared::tile_ops::compute_bounded_tile_offsets(
                                &upper_left,
                                &lower_right,
                            );

                        for (layer_id, layer) in self.canvas.read().await.iter().enumerate() {
                            let mut visible_strokes = BTreeSet::new();
                            for tile_offset in &conn.active_tile_offsets {
                                visible_strokes
                                    .append(&mut layer.get_tile_paintstrokes(&tile_offset));
                            }

                            for stroke in &visible_strokes {
                                // Send paint stroke to everyone connected viewing the visible tiles
                                let msg = ServerMessage::PaintStroke(layer_id as u8, (**stroke).clone());
                                let zbincode_msg = netsketch_shared::to_zbincode(&msg);

                                match zbincode_msg {
                                    Ok(msg) => {
                                        if let Err(err) =
                                            conn.tx_conn.send(Ok(WsMessage::binary(msg)))
                                        {
                                            room_eprintln!(self, "Send error: {}", err.to_string());
                                        }
                                    }
                                    Err(err) => {
                                        room_eprintln!(self, "ZBincode error: {}", err.to_string());
                                    }
                                };
                            }
                        }
                    }
                }
                _ => (),
            }
        }
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
