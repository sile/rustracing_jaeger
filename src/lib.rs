//! [Jaeger][jaeger] client library created on top of [rustracing].
//!
//! [jaeger]: https://github.com/jaegertracing/jaeger
//! [rustracing]: https://crates.io/crates/rustracing
//!
//! # Examples
//!
//! ```
//! # extern crate rustracing;
//! # extern crate rustracing_jaeger;
//! use rustracing::sampler::AllSampler;
//! use rustracing_jaeger::Tracer;
//! use rustracing_jaeger::reporter::JaegerCompactReporter;
//! # fn main() {
//! let (tracer, span_rx) = Tracer::new(AllSampler);
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
//! # }
//! ```

#![warn(missing_docs)]
extern crate hostname;
extern crate rand;
extern crate rustracing;
extern crate thrift_codec;
#[macro_use]
extern crate trackable;

pub use rustracing::{Result, Error, ErrorKind};
pub use span::Span;
pub use tracer::Tracer;

pub mod reporter;
pub mod span;

mod constants;
mod error;
mod thrift;
mod tracer;

#[cfg(test)]
mod tests {
    use rustracing::sampler::AllSampler;
    use rustracing::tag::Tag;

    use Tracer;
    use reporter::JaegerCompactReporter;

    #[test]
    fn it_works() {
        let (tracer, span_rx) = Tracer::new(AllSampler);
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
