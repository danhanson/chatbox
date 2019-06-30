use actix::prelude::*;

#[derive(Message)]
struct ClientMessage(String);
