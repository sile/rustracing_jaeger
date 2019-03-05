use rustracing::sampler::{BoxSampler, Sampler};
use rustracing::Tracer as InnerTracer;
use std::borrow::Cow;
use std::fmt;

use crate::span::{SpanContextState, SpanSender, StartSpanOptions};

/// Tracer.
#[derive(Clone)]
pub struct Tracer {
    inner: InnerTracer<BoxSampler<SpanContextState>, SpanContextState>,
}
impl Tracer {
    /// Makes a new `Tracer` instance.
    pub fn new<S>(sampler: S, span_tx: SpanSender) -> Self
    where
        S: Sampler<SpanContextState> + Send + Sync + 'static,
    {
        let inner = InnerTracer::new(sampler.boxed(), span_tx);
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
        let tracer = Tracer::new(NullSampler, span_tx);
        is_send(tracer);
    }
}
