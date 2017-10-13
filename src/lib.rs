extern crate byteorder;
extern crate rand;
extern crate rustracing;
extern crate thrift_codec;
#[macro_use]
extern crate trackable;

// TODO
pub use thrift_codec::{Result, Error, ErrorKind};

pub mod reporter;
pub mod span;
pub mod tracer;
pub mod thrift;

#[cfg(test)]
mod tests {
    use rustracing::AlwaysSampler;

    use tracer::Tracer;

    #[test]
    fn it_works() {
        let (tracer, span_rx) = Tracer::new(AlwaysSampler);
        {
            let _span = tracer.span("it_works").start();
            // do something
        }
        let _span = span_rx.try_recv().unwrap();
    }
}
