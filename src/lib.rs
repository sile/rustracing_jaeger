extern crate hostname;
extern crate rand;
extern crate rustracing;
extern crate thrift_codec;
#[macro_use]
extern crate trackable;

pub use rustracing::{Result, Error, ErrorKind};
pub use span::Span;
pub use tracer::Tracer;

pub mod constants;
pub mod reporter;
pub mod span;
pub mod thrift;

mod error;
mod tracer;

#[cfg(test)]
mod tests {
    use rustracing::sampler::AllSampler;

    use Tracer;

    #[test]
    fn it_works() {
        let (tracer, span_rx) = Tracer::new(AllSampler);
        {
            let _span = tracer.span("it_works").start();
            // do something
        }
        let span = span_rx.try_recv().unwrap();
        assert_eq!(span.operation_name(), "it_works");
    }
}
