use rustracing::sampler::{BoxSampler, Sampler};
use rustracing::Tracer as InnerTracer;
use std::borrow::Cow;
use std::fmt;

use crate::span::{SpanContextState, SpanReceiver, SpanSender, StartSpanOptions};

/// Tracer.
#[derive(Clone)]
pub struct Tracer {
    inner: InnerTracer<BoxSampler<SpanContextState>, SpanContextState>,
}
impl Tracer {
    /// Makes a new `Tracer` instance with an unbounded channel.
    ///
    /// This constructor is mainly for backward compatibility, it has the same interface
    /// as in previous versions except the type of `SpanReceiver`.
    /// It builds an unbounded channel which may cause memory issues if there is no reader,
    /// prefer `with_sender()` alternative with a bounded one.
    pub fn new<S>(sampler: S) -> (Self, SpanReceiver)
    where
        S: Sampler<SpanContextState> + Send + Sync + 'static,
    {
        let (inner, rx) = InnerTracer::new(sampler.boxed());
        (Tracer { inner }, rx)
    }

    /// Makes a new `Tracer` instance.
    pub fn with_sender<S>(sampler: S, span_tx: SpanSender) -> Self
    where
        S: Sampler<SpanContextState> + Send + Sync + 'static,
    {
        let inner = InnerTracer::with_sender(sampler.boxed(), span_tx);
        Tracer { inner }
    }

    /// Clone with the given `sampler`.
    pub fn clone_with_sampler<T>(&self, sampler: T) -> Self
    where
        T: Sampler<SpanContextState> + Send + Sync + 'static,
    {
        let inner = self.inner.clone_with_sampler(sampler.boxed());
        Tracer { inner }
    }

    /// Returns `StartSpanOptions` for starting a span which has the name `operation_name`.
    pub fn span<N>(&self, operation_name: N) -> StartSpanOptions
    where
        N: Into<Cow<'static, str>>,
    {
        self.inner.span(operation_name)
    }
}
impl fmt::Debug for Tracer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tracer {{ .. }}")
    }
}

#[cfg(test)]
mod test {
    use rustracing::sampler::NullSampler;

    use super::*;

    #[test]
    fn is_tracer_sendable() {
        fn is_send<T: Send>(_: T) {}

        let (span_tx, _span_rx) = crossbeam_channel::bounded(10);
        let tracer = Tracer::with_sender(NullSampler, span_tx);
        is_send(tracer);
    }
}
