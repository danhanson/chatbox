#[macro_use]
extern crate serde_derive;

mod client;
mod client_message;

use actix_files as fs;
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, Error};
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use client::{ClientAddress, ws_client::WsClient};
use actix_web_actors::ws;
use client_message::ClientMessage;
use futures::stream::Stream;
use futures::future::{Future, ok, err, Either, collect};
use std::sync::Arc;
use std::io::Write;
use futures_locks::RwLock;
use serde_json;

#[derive(Serialize)]
struct Room {
    name: String,
    members: HashSet<String>,
    comments: Vec<String>
}

impl Room {
    fn new(name: String) -> Room {
        Room {
            name: name,
            members: HashSet::new(),
            comments: Vec::new()
        }
    }

    fn add_comment(&mut self, comment: String) {
        self.comments.push(comment);
    }
}

struct ChatBox {
    connections: RwLock<HashMap<String, ClientAddress>>,
    rooms: RwLock<HashMap<String, RwLock<Room>>>
}

impl ChatBox {

    fn comment(
        &self,
        room_name: String,
        comment: String
    ) -> impl Future<Item=(), Error=()> {
        self.rooms.read()
            .and_then(move |rooms| {
                if let Some(room_lock) = rooms.get(&room_name) {
                    Either::A(room_lock.write().map(|room| (room, room_name)))
                } else {
                    Either::B(err(()))
                }
            })
            .map(|(mut room, room_name)| {
                room.add_comment(comment.clone());
                (room, comment, room_name)
            })
            .join(self.connections.read())
            .map(|((room, comment, room_name), cons)| {
                for mem in room.members.iter() {
                    if let Some(con) = cons.get(mem) {
                        let message = format!(
                            r#"{{"topic":"comment","room":"{}","comment":"{}"}}"#,
                            room_name,
                            comment
                        );
                        con.send(ClientMessage(message))
                    }
                }
            })
    }

    fn new() -> ChatBox {
        ChatBox {
            connections: RwLock::new(HashMap::new()),
            rooms: RwLock::new(HashMap::new())
        }
    }
}


fn get_socket(
    req: HttpRequest,
    stream: web::Payload,
    _: web::Data<Arc<ChatBox>>
) -> Result<HttpResponse, Error> {
    ws::start(WsClient{}, &req, stream)
}

#[derive(Deserialize)]
pub struct RoomSelection {
    room: String
}

fn post_comment(
    room: web::Path<RoomSelection>,
    stream: web::Payload,
    chat_box: web::Data<Arc<ChatBox>>
) -> impl Future<Item=HttpResponse, Error=Error> {
    stream
        .concat2()
        .map_err(|_| HttpResponse::BadRequest().finish())
        .and_then(|comment| {
            String::from_utf8(comment[..].into()).map_err(|_| HttpResponse::BadRequest().body("Not valid utf8"))
        })
        .map(move |comment| {
            chat_box.comment(room.into_inner().room, comment);
            HttpResponse::Ok().finish()
        })
        .or_else(|e| {
            ok(e)
        })
}

fn get_comments(
    room_selection: web::Path<RoomSelection>,
    chat_box: web::Data<Arc<ChatBox>>
) -> impl Future<Item=HttpResponse, Error=()> {
    chat_box.rooms.read()
        .and_then(move |rooms| {
            if let Some(room_lock) = rooms.get(&room_selection.room) {
                Either::A(
                    room_lock.read()
                        .map(|room| {
                            HttpResponse::Ok().body(
                                serde_json::to_string(&room.comments).unwrap()
                            )
                        })
                )
            } else {
                Either::B(
                    ok(HttpResponse::NotFound().body("That room does not exist"))
                )
            }
        })
}

fn post_room(
    room_selection: web::Path<RoomSelection>,
    chat_box: web::Data<Arc<ChatBox>>
) -> impl Future<Item=HttpResponse, Error=()> {
    let room = room_selection.into_inner().room;
    chat_box.rooms.write()
        .map(|mut rooms| {
            match rooms.entry(room.clone()) {
                Entry::Occupied(_) => HttpResponse::Ok().finish(),
                Entry::Vacant(entry) => {
                    entry.insert(RwLock::new(Room::new(room)));
                    HttpResponse::Created().finish()
                }
            }
        })
}

fn get_room(
    room_selection: web::Path<RoomSelection>,
    chat_box: web::Data<Arc<ChatBox>>
) -> impl Future<Item=HttpResponse, Error=()> {
    chat_box.rooms.read()
        .and_then(move |rooms| {
            if let Some(room_lock) = rooms.get(&room_selection.room) {
                Either::A(
                    room_lock.read()
                        .map(|room| {
                            HttpResponse::Ok().body(
                                serde_json::to_string(&*room).unwrap()
                            )
                        })
                )
            } else {
                Either::B(
                    ok(HttpResponse::NotFound().finish())
                )
            }
        })
}

#[derive(Serialize)]
struct RoomSummary<'a> {
    name: &'a str,
    members: &'a HashSet<String>
}

fn get_room_list(
    chat_box: web::Data<Arc<ChatBox>>
) -> impl Future<Item=HttpResponse, Error=()> {
    chat_box.rooms.read()
        .and_then(|rooms_map| {
            let values: Vec<_> = rooms_map.values().map(|room| room.read()).collect();
            collect(values).map(|rooms| {
                let mut msg = vec![b'['];
                for room in rooms.into_iter() {
                    serde_json::to_writer(
                        msg.by_ref(),
                        &RoomSummary {
                            name: &room.name,
                            members: &room.members
                        }
                    ).unwrap()
                }
                msg.push(b',');
                let last = msg.last_mut().unwrap();
                if *last == b',' {
                    *last = b']';
                } else {
                    msg.push(b']');
                }
                HttpResponse::Ok().body(msg)
            })
        })
}

fn main() -> std::io::Result<()> {
    let chat_box = Arc::new(ChatBox::new());
    HttpServer::new(move|| {
        App::new()
            .data(chat_box.clone())
            .service(
                web::resource("/rooms")
                    .route(web::get().to_async(get_room_list))
            )
            .service(
                web::resource("/room/{room}")
                    .route(web::post().to_async(post_room))
                    .route(web::get().to_async(get_room))
            )
            .service(
                web::resource("/room/{room}/comments")
                    .route(web::post().to_async(post_comment))
                    .route(web::get().to_async(get_comments))
            )
            .service(web::resource("/ws/{room}").to(get_socket))
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
        .bind("127.0.0.1:8080")?
        .run()
}
