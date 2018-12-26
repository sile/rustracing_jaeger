#[macro_use]
extern crate trackable;

use bytecodec::bytes::Utf8Encoder;
use bytecodec::null::NullDecoder;
use fibers_global;
use fibers_http_server::{HandleRequest, Reply, Req, Res, ServerBuilder, Status};
use futures::future::ok;
use httpcodec::{BodyDecoder, BodyEncoder};
use rustracing::sampler::AllSampler;
use rustracing_jaeger::reporter::JaegerCompactReporter;
use rustracing_jaeger::span::SpanContext;
use rustracing_jaeger::Tracer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct Hello {
    tracer: Arc<Mutex<Tracer>>,
}
impl HandleRequest for Hello {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/**";

    type ReqBody = ();
    type ResBody = String;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<Utf8Encoder>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let mut carrier = HashMap::new();
        for field in req.header().fields() {
            carrier.insert(field.name().to_lowercase(), field.value().to_owned());
        }

        let context = track_try_unwrap!(SpanContext::extract_from_text_map(&carrier));
        let tracer = self.tracer.lock().expect("Cannot acquire lock");
        let _span = tracer
            .span("Hello::handle_request")
            .child_of(&context)
            .start();
        let body = format!("Hello: {}\n", req.url().path());
        Box::new(ok(Res::new(Status::Ok, body)))
    }
}

fn main() -> trackable::result::MainResult {
    let (tracer, span_rx) = Tracer::new(AllSampler);
    let handler = Hello {
        tracer: Arc::new(Mutex::new(tracer)),
    };
    std::thread::spawn(move || {
        let reporter = track_try_unwrap!(JaegerCompactReporter::new("http_hello_server"));
        for span in span_rx {
            track_try_unwrap!(reporter.report(&[span]));
        }
    });

    let mut builder = ServerBuilder::new(track_any_err!("127.0.0.1:8081".parse())?);
    track!(builder.add_handler(handler))?;
    let server = builder.finish(fibers_global::handle());
    track!(fibers_global::execute(server))?;
    Ok(())
}
