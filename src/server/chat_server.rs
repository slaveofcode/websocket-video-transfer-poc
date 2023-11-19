use std::collections::{HashMap, HashSet};

use actix::prelude::*;
use rand::{rngs::ThreadRng, Rng};

use crate::session;

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<session::Message>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

// #[derive(Message)]
// #[rtype(result = "()")]
pub struct Message {
    pub id: usize,
    pub msg: String,
    pub room: String,
}

pub struct MessageValue {}

impl actix::Message for Message {
    type Result = MessageValue;
}

pub struct ListRooms;

impl actix::Message for ListRooms {
    type Result = Vec<String>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub id: usize,
    pub name: String,
}

pub struct ChatServer {
    sessions: HashMap<usize, Recipient<session::Message>>,
    rooms: HashMap<String, HashSet<usize>>,
    rng: ThreadRng,
}

impl ChatServer {
    fn send_message(&self, room: &str, message: &str, skip_id: usize) {
        if let Some(sessions) = self.rooms.get(room) {
            for id in sessions {
                println!("sending to client id: {}", &id);
                if *id != skip_id {
                    if let Some(addr) = self.sessions.get(id) {
                        addr.do_send(session::Message(message.to_owned()));
                    }
                }
            }
        }
    }
}

impl Default for ChatServer {
    fn default() -> Self {
        let mut rooms = HashMap::new();
        rooms.insert("main".to_owned(), HashSet::new());

        ChatServer {
            sessions: HashMap::new(),
            rooms,
            rng: rand::thread_rng(),
        }
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _ctx: &mut Self::Context) -> Self::Result {
        println!("Someone joined!");

        // sending to other clients except this server
        self.send_message("main", "Someone Joined", 0);

        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        self.rooms.get_mut("main").unwrap().insert(id);

        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Someone disconnected");

        let mut rooms: Vec<String> = Vec::new();

        // remove address
        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all rooms
            for (name, sessions) in &mut self.rooms {
                if sessions.remove(&msg.id) {
                    rooms.push(name.to_owned());
                }
            }
        }
        // send message to other users
        for room in rooms {
            self.send_message(&room, "Someone disconnected", 0);
        }
    }
}

/// Handler for Message message.
impl Handler<Message> for ChatServer {
    type Result = MessageResult<Message>; 

    fn handle(&mut self, msg: Message, _: &mut Context<Self>) -> Self::Result {
        println!("Someone sending message");
        self.send_message(&msg.room, msg.msg.as_str(), msg.id);
        
        MessageResult(MessageValue {})
    }
}

/// Handler for `ListRooms` message.
impl Handler<ListRooms> for ChatServer {
    type Result = MessageResult<ListRooms>;

    fn handle(&mut self, _: ListRooms, _: &mut Context<Self>) -> Self::Result {
        let mut rooms = Vec::new();

        for key in self.rooms.keys() {
            rooms.push(key.to_owned())
        }

        MessageResult(rooms)
    }
}

/// Join room, send disconnect message to old room
/// send join message to new room
impl Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join { id, name } = msg;
        let mut rooms = Vec::new();

        // remove session from all rooms
        for (n, sessions) in &mut self.rooms {
            if sessions.remove(&id) {
                rooms.push(n.to_owned());
            }
        }
        // send message to other users
        for room in rooms {
            self.send_message(&room, "Someone disconnected", 0);
        }

        if self.rooms.get_mut(&name).is_none() {
            self.rooms.insert(name.clone(), HashSet::new());
        }
        self.send_message(&name, "Someone connected", id);
        self.rooms.get_mut(&name).unwrap().insert(id);
    }
}

