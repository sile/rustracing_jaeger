extern crate rustracing;
extern crate rustracing_jaeger;
#[macro_use]
extern crate trackable;

use std::time::Duration;
use std::thread;
use rustracing::tag::Tag;
use rustracing_jaeger::tracer::Tracer;
use rustracing_jaeger::reporter::JaegerCompactReporter;

fn main() {
    let (tracer, span_rx) = Tracer::new(rustracing::AlwaysSampler);
    {
        let span0 = tracer.span("main").start();
        thread::sleep(Duration::from_millis(10));
        {
            let mut span1 = tracer
                .span("sub")
                .child_of(&span0)
                .tag(Tag::new("foo", "bar"))
                .start();
            span1.log(|log| { log.error().message("something wrong"); });
            thread::sleep(Duration::from_millis(10));
        }
    }

    let mut reporter = track_try_unwrap!(JaegerCompactReporter::new("example"));
    reporter.set_service_tag(Tag::new("hello", "world"));
    track_try_unwrap!(reporter.report(&span_rx.try_iter().collect::<Vec<_>>()));
}
