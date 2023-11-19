#![allow(dead_code)]
use std::io;

use actix::prelude::*;
use actix_codec::{Decoder, Encoder};
use actix_web::web::{BufMut, BytesMut};
use byteorder::{BigEndian, ByteOrder};
use serde::{Deserialize, Serialize};
use serde_json as json;

#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(tag = "cmd", content = "data")]
pub enum ChatRequest {
    List,
    Join(String),
    Message(String),
    Ping,
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(tag = "cmd",content = "data")]
pub enum ChatResponse {
    Ping,
    Rooms(Vec<String>),
    Joined(String),
    Message(String),
}

pub struct ChatCodec;

impl Decoder for ChatCodec {
    type Item = ChatRequest;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }

            BigEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            let _ = src.split_to(2);
            let buf = src.split_to(size);
            return Ok(Some(json::from_slice::<ChatRequest>(&buf)?));
        }

        Ok(None)
    }
}

impl Encoder<ChatResponse> for ChatCodec {
    type Error = io::Error;

    fn encode(&mut self, item: ChatResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&item).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16(msg_ref.len() as u16);
        dst.put(msg_ref);

        Ok(())
    }
}

pub struct ClientChatCodec;

impl Decoder for ClientChatCodec {
    type Item = ChatResponse;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }

            BigEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            let _ = src.split_to(2);
            let buf = src.split_to(size);
            return Ok(Some(json::from_slice::<ChatResponse>(&buf)?));
        }

        Ok(None)
    }
}

impl Encoder<ChatRequest> for ClientChatCodec {
    type Error = io::Error;

    fn encode(&mut self, item: ChatRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&item).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16(msg_ref.len() as u16);
        dst.put(msg_ref);

        Ok(())
    }
}

