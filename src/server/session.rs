use std::{
    io, net, str::FromStr,
    time::{Duration, Instant},
};

use actix::{prelude::*, spawn};
use tokio::{
    io::{split, WriteHalf},
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::FramedRead;

use shared::codec::{ChatCodec, ChatRequest, ChatResponse};

use crate::chat_server::{ChatServer, Connect, ListRooms, Disconnect, Message as ChatMessage};

const WEBSOCKET_ADDR: &str = "0.0.0.0";
const WEBSOCKET_PORT: u16 = 3131;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

pub struct ChatSession {
    id: usize,
    addr: Addr<ChatServer>,
    hb: Instant,
    room: String,
    framed: actix::io::FramedWrite<ChatResponse, WriteHalf<TcpStream>, ChatCodec>,
}

impl Actor for ChatSession {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    _ => ctx.stop(),
                }

                actix::fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send::<Disconnect>(Disconnect { id: self.id });
        Running::Stop
    }
}

impl actix::io::WriteHandler<io::Error> for ChatSession {}

impl StreamHandler<Result<ChatRequest, io::Error>> for ChatSession {
    fn handle(&mut self, msg: Result<ChatRequest, io::Error>, ctx: &mut Context<Self>) {
        match msg {
            Ok(ChatRequest::List) => {
                println!("List of rooms!");
                self.addr
                    .send::<ListRooms>(ListRooms)
                    .into_actor(self)
                    .then(|res, act, _| {
                        match res {
                            Ok(rooms) => act.framed.write(ChatResponse::Rooms(rooms)),
                            _ => println!("something get wrong!"),
                        }
                        actix::fut::ready(())
                    })
                    .wait(ctx)
            },
            Ok(ChatRequest::Join(name)) => {
                println!("join to {}", &name);
                self.room = name;
            },
            Ok(ChatRequest::Message(name)) => {
                println!("sending message {}", &name);
                self.addr
                    .send::<ChatMessage>(ChatMessage {
                        id: 0,
                        room: self.room.to_string(),
                        msg: name,
                    })
                    .into_actor(self)
                    .then::<_, _>(|_, _, _: &mut Context<ChatSession>| {
                        actix::fut::ready(())
                    })
                    .wait(ctx);
            },
            Ok(ChatRequest::Ping) => self.hb = Instant::now(),
            _ => ctx.stop(),
        }
    }
}

impl Handler<Message> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: Message, _ctx: &mut Self::Context) -> Self::Result {
        self.framed.write(ChatResponse::Message(msg.0));
    }
}

impl ChatSession {
    pub fn new(
        addr: Addr<ChatServer>,
        framed: actix::io::FramedWrite<ChatResponse, WriteHalf<TcpStream>, ChatCodec>,
    ) -> ChatSession {
        ChatSession {
            id: 0,
            addr,
            hb: Instant::now(),
            room: "main".to_owned(),
            framed,
        }
    }

    pub fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::new(1, 0), |act, ctx| {
            if Instant::now().duration_since(act.hb) > Duration::new(10, 0) {
                // heartbeat timed out
                println!("Client heartbeat failed, disconnecting!");

                // notify chat server
                act.addr.do_send::<Disconnect>(Disconnect { id: act.id });

                // stop actor
                ctx.stop();
            }

            act.framed.write(ChatResponse::Ping);
        });
    }
}

/// Define TCP server that will accept incoming TCP connection and create
/// chat actors.
pub fn tcp_server(_s: &str, server: Addr<ChatServer>) {
    // Create server listener
    let sock_addr = format!("{}:{}", WEBSOCKET_ADDR, WEBSOCKET_PORT.to_string().as_str()).to_string();
    let addr = net::SocketAddr::from_str(&sock_addr).unwrap();

    spawn(async move {
        let listener = TcpListener::bind(&addr).await.unwrap();

        while let Ok((stream, _)) = listener.accept().await {
            let server = server.clone();
            ChatSession::create(|ctx| {
                let (r, w) = split(stream);
                ChatSession::add_stream(FramedRead::new(r, ChatCodec), ctx);
                ChatSession::new(server, actix::io::FramedWrite::new(w, ChatCodec, ctx))
            });
        }
    });
}
