rustracing_jaeger
=================

- https://github.com/uber/jaeger-idl/blob/master/thrift/agent.thrift
- https://github.com/uber/jaeger-idl/blob/master/thrift/jaeger.thrift

Examples
--------

```console
$ docker run -d -p6831:6831/udp -p6832:6832/udp -p16686:16686 jaegertracing/all-in-one:latest

$ cargo run --example report
$ firefox http://localhost:16686/
```
