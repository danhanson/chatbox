use actix::prelude::*;

#[derive(Message)]
pub struct ClientMessage(pub String);
