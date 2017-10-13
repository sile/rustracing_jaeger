use std::ops::{Deref, DerefMut};
use rustracing::{self, Sampler};

use span::SpanContext;

#[derive(Clone)]
pub struct Tracer {
    inner: rustracing::Tracer<Box<Sampler<SpanContext> + Send + 'static>, SpanContext>,
}
impl Tracer {
    pub fn new<S>(sampler: S) -> (Self, rustracing::span::SpanReceiver<SpanContext>)
    where
        S: Sampler<SpanContext> + Send + 'static,
    {
        let sampler: Box<Sampler<_> + Send + 'static> = Box::new(sampler);
        let (inner, rx) = rustracing::Tracer::new(sampler);
        (Tracer { inner }, rx)
    }
}
impl Deref for Tracer {
    type Target = rustracing::Tracer<Box<Sampler<SpanContext> + Send + 'static>, SpanContext>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for Tracer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
