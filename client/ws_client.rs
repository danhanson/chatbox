/*
 * Even though we are using a websocket, we are only using it to push notifications.
 */
use crate::client_message::ClientMessage;
use actix::prelude::*;
use actix_web_actors::ws;

pub struct WsClient;

impl Actor for WsClient {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsClient {

    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Text(text) => ctx.text(format!("Unexpected Message: {}", text)),
            ws::Message::Binary(_) => ctx.text("Invalid Message"),
            ws::Message::Close(reason) => ctx.close(reason),
            _ => ()
        }
    }
}

impl Handler<ClientMessage> for WsClient {
    type Result = ();
    fn handle(&mut self, ref msg: ClientMessage, ctx: &mut Self::Context) {
        let ClientMessage(string) = msg;
        ctx.text(string)
    }
}
