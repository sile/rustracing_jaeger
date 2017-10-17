//! https://github.com/uber/jaeger-client-go/tree/v2.9.0/constants.go
#![allow(dead_code)]

/// The version of the client library reported as Span tag.
pub const JAEGER_CLIENT_VERSION: &str = concat!("rustracing_jaeger-", env!("CARGO_PKG_VERSION"));

/// The name of the tag used to report client version.
pub const JAEGER_CLIENT_VERSION_TAG_KEY: &str = "jaeger.version";

/// The name of HTTP header or a `TextMap` carrier key which,
/// if found in the carrier, forces the trace to be sampled as "debug" trace.
///
/// The value of the header is recorded as the tag on the root span, so that the
/// trace can be found in the UI using this value as a correlation ID.
pub const JAEGER_DEBUG_HEADER: &str = "jaeger-debug-id";

/// The name of the HTTP header that is used to submit baggage.
///
/// It differs from `TRACE_BAGGAGE_HEADER_PREFIX` in that it can be used only in cases where
/// a root span does not exist.
pub const JAEGER_BAGGAGE_HEADER: &str = "jaeger-baggage";

/// This is used to report host name of the process.
pub const TRACER_HOSTNAME_TAG_KEY: &str = "hostname";

/// This is used to report ip of the process.
pub const TRACER_IP_TAG_KEY: &str = "ip";

/// The http header name used to propagate tracing context.
///
/// This must be in lower-case to avoid mismatches when decoding incoming headers.
pub const TRACER_CONTEXT_HEADER_NAME: &str = "uber-trace-id";

/// The prefix for http headers used to propagate baggage.
///
/// This must be in lower-case to avoid mismatches when decoding incoming headers.
pub const TRACE_BAGGAGE_HEADER_PREFIX: &str = "uberctx-";
