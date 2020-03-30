//! Thrift components defined in [agent.thrift].
//!
//! [agent.thrift]: https://github.com/uber/jaeger-idl/blob/master/thrift/agent.thrift.
use thrift_codec::data::Struct;
use thrift_codec::message::Message;

use crate::thrift::jaeger::Batch;

/// `emitBatch` message defined in [agent.thrift].
///
/// [agent.thrift]: https://github.com/uber/jaeger-idl/blob/master/thrift/agent.thrift]
#[derive(Debug, Clone)]
pub struct EmitBatchNotification {
    /// `batch` argument.
    pub batch: Batch,
}
impl From<EmitBatchNotification> for Message {
    fn from(f: EmitBatchNotification) -> Self {
        Message::oneway("emitBatch", 0, Struct::from((Struct::from(f.batch),)))
    }
}
