pub mod ws_client;

use crate::client_message::ClientMessage;
use actix::prelude::Addr;
use ws_client::WsClient;

#[derive(Clone)]
pub enum ClientAddress {
    WS(Addr<WsClient>)
}

impl ClientAddress {
    pub fn send(&self, msg: ClientMessage) {
        match self {
            ClientAddress::WS(addr) => addr.do_send(msg)
        }
    }
}
