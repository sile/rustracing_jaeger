use std::net::{UdpSocket, SocketAddr};
use rustracing::tag::Tag;
use span::FinishedSpan;
use thrift_codec::CompactEncode;
use thrift_codec::message::Message;

use {Result, Error};
use thrift::{agent, jaeger};

#[derive(Debug)]
pub struct JaegerCompactReporter {
    socket: UdpSocket,
    agent: SocketAddr,
    process: jaeger::Process,
}
impl JaegerCompactReporter {
    pub fn new(service_name: &str) -> Result<Self> {
        let socket = track!(UdpSocket::bind("127.0.0.1:0").map_err(Error::from))?;
        let process = jaeger::Process {
            service_name: service_name.to_owned(),
            tags: Vec::new(),
        };
        let agent = SocketAddr::from(([127, 0, 0, 1], 6831));
        Ok(JaegerCompactReporter {
            socket,
            agent,
            process,
        })
    }
    pub fn set_agent_addr(&mut self, addr: SocketAddr) {
        self.agent = addr;
    }
    pub fn set_service_tag(&mut self, tag: Tag) {
        self.process.tags.push((&tag).into());
    }
    pub fn report(&self, spans: &[FinishedSpan]) -> Result<()> {
        let batch = jaeger::Batch {
            process: self.process.clone(),
            spans: spans.iter().map(From::from).collect(),
        };
        let message = Message::from(agent::EmitBatchNotification { batch });

        let mut bytes = Vec::new();
        track!(message.compact_encode(&mut bytes))?;
        track!(self.socket.send_to(&bytes, self.agent).map_err(Error::from))?;
        Ok(())
    }
}
