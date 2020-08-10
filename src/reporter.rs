//! Reporter to the [jaeger agent]
//!
//! [jaeger agent]: http://jaeger.readthedocs.io/en/latest/deployment/#agent
use crate::constants;
use crate::error;
use crate::span::FinishedSpan;
use crate::thrift::{agent, jaeger};
use crate::Result;
use rustracing::tag::Tag;
use std::net::{SocketAddr, UdpSocket};
use thrift_codec::message::Message;
use thrift_codec::{BinaryEncode, CompactEncode};

/// Reporter for the agent which accepts jaeger.thrift over compact thrift protocol.
#[derive(Debug)]
pub struct JaegerCompactReporter(JaegerReporter);
impl JaegerCompactReporter {
    /// Makes a new `JaegerCompactReporter` instance.
    ///
    /// # Errors
    ///
    /// If the UDP socket used to report spans can not be bound to `0.0.0.0:0`,
    /// it will return an error which has the kind `ErrorKind::Other`.
    pub fn new(service_name: &str) -> Result<Self> {
        let inner = track!(JaegerReporter::new(service_name, 6831))?;
        Ok(JaegerCompactReporter(inner))
    }

    /// Sets the address of the report destination agent to `addr`.
    ///
    /// The default address is `127.0.0.1:6831`.
    ///
    /// Note that you may also need to call `set_reporter_addr` if the `addr` is IPv6 or non localhost address.
    pub fn set_agent_addr(&mut self, addr: SocketAddr) {
        self.0.set_agent_addr(addr);
    }

    /// Sets the address to which the reporter bind.
    ///
    /// The default address is `127.0.0.1:0`.
    pub fn set_reporter_addr(&mut self, addr: SocketAddr) -> Result<()> {
        track!(self.0.set_reporter_addr(addr))
    }

    /// Adds `tag` to this service.
    pub fn add_service_tag(&mut self, tag: Tag) {
        self.0.add_service_tag(tag);
    }

    /// Reports `spans`.
    ///
    /// # Errors
    ///
    /// If it fails to encode `spans` to the thrift compact format (i.e., a bug of this crate),
    /// this method will return an error which has the kind `ErrorKind::InvalidInput`.
    ///
    /// If it fails to send the encoded binary to the jaeger agent via UDP,
    /// this method will return an error which has the kind `ErrorKind::Other`.
    pub fn report(&self, spans: &[FinishedSpan]) -> Result<()> {
        track!(self.0.report(spans, |message| {
            let mut bytes = Vec::new();
            track!(message
                .compact_encode(&mut bytes)
                .map_err(error::from_thrift_error))?;
            Ok(bytes)
        }))
    }
}

/// Reporter for the agent which accepts jaeger.thrift over binary thrift protocol.
#[derive(Debug)]
pub struct JaegerBinaryReporter(JaegerReporter);
impl JaegerBinaryReporter {
    /// Makes a new `JaegerBinaryReporter` instance.
    ///
    /// # Errors
    ///
    /// If the UDP socket used to report spans can not be bound to `0.0.0.0:0`,
    /// it will return an error which has the kind `ErrorKind::Other`.
    pub fn new(service_name: &str) -> Result<Self> {
        let inner = track!(JaegerReporter::new(service_name, 6832))?;
        Ok(JaegerBinaryReporter(inner))
    }

    /// Sets the address of the report destination agent to `addr`.
    ///
    /// The default address is `127.0.0.1:6832`.
    ///
    /// Note that you may also need to call `set_reporter_addr` if the `addr` is IPv6 or non localhost address.
    pub fn set_agent_addr(&mut self, addr: SocketAddr) {
        self.0.set_agent_addr(addr);
    }

    /// Sets the address to which the report bind.
    ///
    /// The default address is `127.0.0.1:0`.
    pub fn set_reporter_addr(&mut self, addr: SocketAddr) -> Result<()> {
        track!(self.0.set_reporter_addr(addr))
    }

    /// Adds `tag` to this service.
    pub fn add_service_tag(&mut self, tag: Tag) {
        self.0.add_service_tag(tag);
    }

    /// Reports `spans`.
    ///
    /// # Errors
    ///
    /// If it fails to encode `spans` to the thrift binary format (i.e., a bug of this crate),
    /// this method will return an error which has the kind `ErrorKind::InvalidInput`.
    ///
    /// If it fails to send the encoded binary to the jaeger agent via UDP,
    /// this method will return an error which has the kind `ErrorKind::Other`.
    pub fn report(&self, spans: &[FinishedSpan]) -> Result<()> {
        track!(self.0.report(spans, |message| {
            let mut bytes = Vec::new();
            track!(message
                .binary_encode(&mut bytes)
                .map_err(error::from_thrift_error))?;
            Ok(bytes)
        }))
    }
}

#[derive(Debug)]
struct JaegerReporter {
    socket: UdpSocket,
    agent: SocketAddr,
    process: jaeger::Process,
}
impl JaegerReporter {
    fn new(service_name: &str, port: u16) -> Result<Self> {
        let agent = SocketAddr::from(([127, 0, 0, 1], port));
        let socket =
            track!(UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
                .map_err(error::from_io_error))?;
        let process = jaeger::Process {
            service_name: service_name.to_owned(),
            tags: Vec::new(),
        };
        let mut this = JaegerReporter {
            socket,
            agent,
            process,
        };

        this.add_service_tag(Tag::new(
            constants::JAEGER_CLIENT_VERSION_TAG_KEY,
            constants::JAEGER_CLIENT_VERSION,
        ));
        if let Ok(Ok(hostname)) = hostname::get().map(|h| h.into_string()) {
            this.add_service_tag(Tag::new(constants::TRACER_HOSTNAME_TAG_KEY, hostname));
        }
        Ok(this)
    }
    fn set_agent_addr(&mut self, addr: SocketAddr) {
        self.agent = addr;
    }
    fn set_reporter_addr(&mut self, addr: SocketAddr) -> Result<()> {
        self.socket = track!(UdpSocket::bind(addr).map_err(error::from_io_error))?;
        Ok(())
    }
    fn add_service_tag(&mut self, tag: Tag) {
        self.process.tags.push((&tag).into());
    }
    fn report<F>(&self, spans: &[FinishedSpan], encode: F) -> Result<()>
    where
        F: FnOnce(Message) -> Result<Vec<u8>>,
    {
        let batch = jaeger::Batch {
            process: self.process.clone(),
            spans: spans.iter().map(From::from).collect(),
        };
        let message = Message::from(agent::EmitBatchNotification { batch });
        let bytes = track!(encode(message))?;
        track!(self
            .socket
            .send_to(&bytes, self.agent)
            .map_err(error::from_io_error))?;
        Ok(())
    }
}
