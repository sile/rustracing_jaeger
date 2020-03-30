//! [Jaeger][jaeger] client library created on top of [rustracing].
//!
//! [jaeger]: https://github.com/jaegertracing/jaeger
//! [rustracing]: https://crates.io/crates/rustracing
//!
//! # Examples
//!
//! ```
//! use rustracing::sampler::AllSampler;
//! use rustracing_jaeger::Tracer;
//! use rustracing_jaeger::reporter::JaegerCompactReporter;
//!
//! // Creates a tracer
//! let (span_tx, span_rx) = crossbeam_channel::bounded(10);
//! let tracer = Tracer::with_sender(AllSampler, span_tx);
//! {
//!     let span = tracer.span("sample_op").start();
//!     // Do something
//!
//! } // The dropped span will be sent to `span_rx`
//!
//! let span = span_rx.try_recv().unwrap();
//! assert_eq!(span.operation_name(), "sample_op");
//!
//! // Reports this span to the local jaeger agent
//! let reporter = JaegerCompactReporter::new("sample_service").unwrap();
//! reporter.report(&[span]).unwrap();
//! ```

#![warn(missing_docs)]
#[macro_use]
extern crate trackable;

pub use self::span::Span;
pub use self::tracer::Tracer;
pub use rustracing::{Error, ErrorKind, Result};

pub mod reporter;
pub mod span;
pub mod thrift;

mod constants;
mod error;
mod tracer;

#[cfg(test)]
mod tests {
    use crate::reporter::JaegerCompactReporter;
    use crate::Tracer;
    use rustracing::sampler::AllSampler;
    use rustracing::tag::Tag;

    #[test]
    fn it_works() {
        let (span_tx, span_rx) = crossbeam_channel::bounded(10);
        let tracer = Tracer::with_sender(AllSampler, span_tx);
        {
            let _span = tracer.span("it_works").start();
            // do something
        }
        let span = span_rx.try_recv().unwrap();
        assert_eq!(span.operation_name(), "it_works");

        let mut reporter = JaegerCompactReporter::new("sample_service").unwrap();
        reporter.add_service_tag(Tag::new("foo", "bar"));
        reporter.report(&[span]).unwrap();
    }
}
