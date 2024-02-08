pub mod util;

use std::{collections::HashMap, pin::Pin, time::Duration};

use crate::{
    api::socket::util::{ResultType, SocketRequest, SocketResponse},
    simulation::blocks::{Coords, SourceDest},
    validator::{game_handler, state::State},
};
use actix::prelude::*;
use actix_web_actors::ws;
use futures::FutureExt;
use serde_json;
// use std::{task::{Poll}, thread};

pub struct Socket {
    pub game_id: i32,
    pub game_state: State, // Has buildings, mines too
    pub shortest_paths: HashMap<SourceDest, Coords>,
    // pub timer_handle: Option<Pin<Box<actix::clock::Sleep>>>,
}

impl Actor for Socket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // self.timer_handle = Some(Box::pin(actix::clock::sleep(Duration::from_secs(120))));

        println!("Websocket started");
        ctx.text("Websocket started");

        let response = SocketResponse {
            frame_number: 0,
            result_type: ResultType::GameOver,
            is_alive: None,
            attacker_health: None,
            exploded_mines: Vec::new(),
            triggered_defenders: Vec::new(),
            defender_damaged: None,
            damaged_buildings: Vec::new(),
            artifacts_gained: Vec::new(),
            is_sync: false,
            state: None,
            is_game_over: true,
            message: None,
        };
        if let Ok(json_response) = serde_json::to_string(&response) {
            ctx.text(json_response);
        } else {
            println!("Error serializing JSON");
            ctx.text("Error serializing JSON");
        }
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        // if let Some(handle) = self.timer_handle.take() {
            // Convert back to a heap-allocated Sleep object
            // let val = match handle.as_mut().poll_unpin(ctx) {
            //     Poll::Pending => thread::park(),
            //     Poll::Ready(res) => return res,
            // };
            // match handle.as_mut().poll_unpin(cx) {
            //     Poll::Pending => thread::park(),
            //     Poll::Ready(res) => return res,
            // }
            // let _ = Box::pin(handle.);
        // }
        println!("Websocket stopped");
        ctx.text("Websocket stopped");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Socket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                println!("Received JSON message: {}", text);
                if let Ok(request) = serde_json::from_str::<SocketRequest>(&text) {
                    println!("Parsed JSON message: {:?}", request);
                    let response = game_handler(0, &request);
                    if response.is_ok() {
                        let response = response.unwrap();
                        if let Ok(json_response) = serde_json::to_string(&response) {
                            ctx.text(json_response);
                        } else {
                            println!("Error serializing JSON");
                            ctx.text("Error serializing JSON");
                        }
                    } else {
                        println!("Error handling game");
                        ctx.text("Error handling game");
                    }
                } else {
                    println!("Error parsing JSON");
                    ctx.text("Error parsing JSON");
                }
            }
            _ => (),
        }
    }
}

