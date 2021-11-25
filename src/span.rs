//! Span.
//!
//! # How to inject/extract a span
//!
//! You can inject/extract the context of a span by using `SpanContext::inject_to_xxx` and
//! `SpanContext::extract_from_xxx` methods respectively.
//!
//! The simplest way is to use `HashMap` as the carrier as follows:
//!
//! ```
//! use std::collections::HashMap;
//! use rustracing_jaeger::span::SpanContext;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Extraction
//! let mut carrier = HashMap::new();
//! carrier.insert(
//!     "uber-trace-id".to_string(),  // NOTE: The key must be lower-case
//!     "6309ab92c95468edea0dc1a9772ae2dc:409423a204bc17a8:0:1".to_string(),
//! );
//! let context = SpanContext::extract_from_text_map(&carrier)?.unwrap();
//! let trace_id = context.state().trace_id();
//! assert_eq!(trace_id.to_string(), "6309ab92c95468edea0dc1a9772ae2dc");
//!
//! // Injection
//! let mut injected_carrier = HashMap::new();
//! context.inject_to_text_map(&mut injected_carrier)?;
//! assert_eq!(injected_carrier, carrier);
//! # Ok(())
//! # }
//! ```
//!
//! # References
//!
//! - [constants.go](https://github.com/uber/jaeger-client-go/tree/v2.9.0/constants.go)
//! - [context.go](https://github.com/uber/jaeger-client-go/tree/v2.9.0/context.go)
//! - [propagation.go](https://github.com/uber/jaeger-client-go/tree/v2.9.0/propagation.go)
use crate::constants;
use crate::error;
use crate::{Error, ErrorKind, Result};
use percent_encoding::percent_decode;
use rustracing::carrier::{
    ExtractFromBinary, ExtractFromHttpHeader, ExtractFromTextMap, InjectToBinary,
    InjectToHttpHeader, InjectToTextMap, IterHttpHeaderFields, SetHttpHeaderField, TextMap,
};
use rustracing::sampler::BoxSampler;
use std::fmt;
use std::io::{Read, Write};
use std::str::{self, FromStr};

/// Span.
pub type Span = rustracing::span::Span<SpanContextState>;

/// Span handle.
pub type SpanHandle = rustracing::span::SpanHandle<SpanContextState>;

/// Finished span.
pub type FinishedSpan = rustracing::span::FinishedSpan<SpanContextState>;

/// Span receiver.
pub type SpanReceiver = rustracing::span::SpanReceiver<SpanContextState>;

/// Sender of finished spans to the destination channel.
pub type SpanSender = rustracing::span::SpanSender<SpanContextState>;

/// Options for starting a span.
pub type StartSpanOptions<'a> =
    rustracing::span::StartSpanOptions<'a, BoxSampler<SpanContextState>, SpanContextState>;

/// Candidate span for tracing.
pub type CandidateSpan<'a> = rustracing::span::CandidateSpan<'a, SpanContextState>;

/// Span context.
pub type SpanContext = rustracing::span::SpanContext<SpanContextState>;

/// Span reference.
pub type SpanReference = rustracing::span::SpanReference<SpanContextState>;

const FLAG_SAMPLED: u8 = 0b01;
const FLAG_DEBUG: u8 = 0b10;

/// Unique 128bit identifier of a trace.
///
/// ```
/// use rustracing_jaeger::span::TraceId;
///
/// let id = TraceId{ high: 0, low: 10 };
/// assert_eq!(id.to_string(), "a");
/// assert_eq!("a".parse::<TraceId>().unwrap(), id);
///
/// let id = TraceId{ high: 1, low: 2 };
/// assert_eq!(id.to_string(), "10000000000000002");
/// assert_eq!("10000000000000002".parse::<TraceId>().unwrap(), id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
pub struct TraceId {
    pub high: u64,
    pub low: u64,
}
impl TraceId {
    /// Makes a randomly generated `TraceId`.
    pub fn new() -> Self {
        TraceId::default()
    }
}
impl Default for TraceId {
    /// Makes a randomly generated `TraceId`.
    fn default() -> Self {
        TraceId {
            high: rand::random(),
            low: rand::random(),
        }
    }
}
impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.high == 0 {
            write!(f, "{:x}", self.low)
        } else {
            write!(f, "{:x}{:016x}", self.high, self.low)
        }
    }
}
impl FromStr for TraceId {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        if s.len() <= 16 {
            let low = track!(u64::from_str_radix(s, 16).map_err(error::from_parse_int_error,))?;
            Ok(TraceId { high: 0, low })
        } else if s.len() <= 32 {
            let (high, low) = s.as_bytes().split_at(s.len() - 16);
            let high = track!(str::from_utf8(high).map_err(error::from_utf8_error))?;
            let high = track!(u64::from_str_radix(high, 16).map_err(error::from_parse_int_error,))?;

            let low = track!(str::from_utf8(low).map_err(error::from_utf8_error))?;
            let low = track!(u64::from_str_radix(low, 16).map_err(error::from_parse_int_error,))?;
            Ok(TraceId { high, low })
        } else {
            track_panic!(ErrorKind::InvalidInput, "s={:?}", s)
        }
    }
}

/// `SpanContextState` builder.
///
/// Normally it is recommended to build `SpanContextState` using APIs provided by `Tracer` or `SpanContext`
/// rather than via this.
///
/// But it may be useful, for example,
/// if you want to handle custom carrier formats that are not defined in the OpenTracing [specification].
///
/// [specification]: https://github.com/opentracing/specification/blob/master/specification.md
#[derive(Debug, Clone)]
pub struct SpanContextStateBuilder {
    trace_id: Option<TraceId>,
    span_id: Option<u64>,
    flags: u8,
    debug_id: String,
}
impl SpanContextStateBuilder {
    /// Makes a new `SpanContextStateBuilder` instance.
    pub fn new() -> Self {
        SpanContextStateBuilder {
            trace_id: None,
            span_id: None,
            flags: FLAG_SAMPLED,
            debug_id: String::new(),
        }
    }

    /// Sets the trace identifier.
    ///
    /// The default value is `TraceId::new()`.
    pub fn trace_id(mut self, trace_id: TraceId) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// Sets the span identifier.
    ///
    /// The default value is `rand::random()`.
    pub fn span_id(mut self, span_id: u64) -> Self {
        self.span_id = Some(span_id);
        self
    }

    /// Sets the debug identifier.
    ///
    /// It is not set by default.
    pub fn debug_id(mut self, debug_id: String) -> Self {
        if !debug_id.is_empty() {
            self.flags |= FLAG_DEBUG;
            self.debug_id = debug_id;
        }
        self
    }

    /// Builds a `SpanContextState` instance with the specified parameters.
    pub fn finish(self) -> SpanContextState {
        SpanContextState {
            trace_id: self.trace_id.unwrap_or_default(),
            span_id: self.span_id.unwrap_or_else(rand::random),
            flags: self.flags,
            debug_id: self.debug_id,
        }
    }
}
impl Default for SpanContextStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Jaeger specific span context state.
#[derive(Debug, Clone)]
pub struct SpanContextState {
    trace_id: TraceId,
    span_id: u64,
    flags: u8,
    debug_id: String,
}
impl SpanContextState {
    /// Returns the trace identifier of this span.
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// Returns the identifier of this span.
    pub fn span_id(&self) -> u64 {
        self.span_id
    }

    /// Returns `true` if this span has been sampled (i.e., being traced).
    pub fn is_sampled(&self) -> bool {
        (self.flags & FLAG_SAMPLED) != 0
    }

    /// Returns the debug identifier of this span if exists.
    pub fn debug_id(&self) -> Option<&str> {
        if self.debug_id.is_empty() {
            None
        } else {
            Some(&self.debug_id)
        }
    }

    fn set_debug_id(&mut self, debug_id: String) {
        if !debug_id.is_empty() {
            self.flags |= FLAG_DEBUG;
            self.debug_id = debug_id;
        }
    }

    /// Returns the flags for this span.
    pub fn flags(&self) -> u8 {
        self.flags
    }

    fn root() -> Self {
        Self::with_trace_id(TraceId::default())
    }

    fn with_trace_id(trace_id: TraceId) -> Self {
        SpanContextState {
            trace_id,
            span_id: rand::random(),
            flags: FLAG_SAMPLED,
            debug_id: String::new(),
        }
    }
}
impl fmt::Display for SpanContextState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dummy_parent_id = 0;
        write!(
            f,
            "{}:{:x}:{:x}:{:x}",
            self.trace_id, self.span_id, dummy_parent_id, self.flags
        )
    }
}
impl FromStr for SpanContextState {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut tokens = s.splitn(4, ':');

        macro_rules! token {
            () => {
                track_assert_some!(tokens.next(), ErrorKind::InvalidInput)
            };
        }
        let trace_id = track!(token!().parse())?;
        let span_id =
            track!(u64::from_str_radix(token!(), 16).map_err(error::from_parse_int_error))?;
        let _parent_span_id =
            track!(u64::from_str_radix(token!(), 16).map_err(error::from_parse_int_error))?;
        let flags = track!(u8::from_str_radix(token!(), 16).map_err(error::from_parse_int_error))?;

        Ok(SpanContextState {
            trace_id,
            span_id,
            flags,
            debug_id: String::new(),
        })
    }
}
impl<'a> From<CandidateSpan<'a>> for SpanContextState {
    fn from(f: CandidateSpan<'a>) -> Self {
        if let Some(primary) = f.references().first() {
            Self::with_trace_id(primary.span().trace_id)
        } else {
            Self::root()
        }
    }
}
impl<T: TextMap> InjectToTextMap<T> for SpanContextState {
    fn inject_to_text_map(context: &SpanContext, carrier: &mut T) -> Result<()> {
        // TODO: Support baggage items
        carrier.set(
            constants::TRACER_CONTEXT_HEADER_NAME,
            &context.state().to_string(),
        );
        Ok(())
    }
}
impl<T: TextMap> ExtractFromTextMap<T> for SpanContextState {
    fn extract_from_text_map(carrier: &T) -> Result<Option<SpanContext>> {
        use std::collections::HashMap;

        // FIXME: optimize
        let mut map = HashMap::new();
        if let Some(v) = carrier.get(constants::TRACER_CONTEXT_HEADER_NAME) {
            map.insert(constants::TRACER_CONTEXT_HEADER_NAME, v);
        }
        if let Some(v) = carrier.get(constants::JAEGER_DEBUG_HEADER) {
            map.insert(constants::JAEGER_DEBUG_HEADER, v);
        }
        track!(Self::extract_from_http_header(&map))
    }
}
impl<T> InjectToHttpHeader<T> for SpanContextState
where
    T: SetHttpHeaderField,
{
    fn inject_to_http_header(context: &SpanContext, carrier: &mut T) -> Result<()> {
        // TODO: Support baggage items
        track!(carrier.set_http_header_field(
            constants::TRACER_CONTEXT_HEADER_NAME,
            &context.state().to_string(),
        ))?;
        Ok(())
    }
}
impl<'a, T> ExtractFromHttpHeader<'a, T> for SpanContextState
where
    T: IterHttpHeaderFields<'a>,
{
    fn extract_from_http_header(carrier: &'a T) -> Result<Option<SpanContext>> {
        let mut state: Option<SpanContextState> = None;
        let mut debug_id = None;
        let baggage_items = Vec::new(); // TODO: Support baggage items
        for (name, value) in carrier.fields() {
            if name.eq_ignore_ascii_case(constants::TRACER_CONTEXT_HEADER_NAME) {
                let value = percent_decode(value);
                let value = track!(value.decode_utf8().map_err(error::from_utf8_error))?;
                state = Some(track!(value.parse())?);
            } else if name.eq_ignore_ascii_case(constants::JAEGER_DEBUG_HEADER) {
                let value = track!(str::from_utf8(value).map_err(error::from_utf8_error))?;
                debug_id = Some(value.to_owned());
            }
        }
        if let Some(mut state) = state {
            if let Some(debug_id) = debug_id.take() {
                state.set_debug_id(debug_id);
            }
            Ok(Some(SpanContext::new(state, baggage_items)))
        } else if let Some(debug_id) = debug_id.take() {
            let state = SpanContextState {
                trace_id: TraceId { high: 0, low: 0 },
                span_id: 0,
                flags: FLAG_DEBUG,
                debug_id,
            };
            Ok(Some(SpanContext::new(state, Vec::new())))
        } else {
            Ok(None)
        }
    }
}
impl<T> InjectToBinary<T> for SpanContextState
where
    T: Write,
{
    fn inject_to_binary(context: &SpanContext, carrier: &mut T) -> Result<()> {
        let mut u64buf: [u8; 8];
        let u32buf: [u8; 4];
        let u8buf: [u8; 1];

        u64buf = context.state().trace_id.high.to_be_bytes();
        track!(carrier.write(&u64buf).map_err(error::from_io_error))?;
        u64buf = context.state().trace_id.low.to_be_bytes();
        track!(carrier.write(&u64buf).map_err(error::from_io_error))?;
        u64buf = context.state().span_id.to_be_bytes();
        track!(carrier.write(&u64buf).map_err(error::from_io_error))?;
        // parent_span_id attribute is obsolete, write zeros.
        u64buf = [0; 8];
        track!(carrier.write(&u64buf).map_err(error::from_io_error))?;
        u8buf = [context.state().flags as u8];
        track!(carrier.write(&u8buf).map_err(error::from_io_error))?;
        // TODO: Support baggage items
        u32buf = [0; 4];
        track!(carrier.write(&u32buf).map_err(error::from_io_error))?;

        Ok(())
    }
}
impl<T> ExtractFromBinary<T> for SpanContextState
where
    T: Read,
{
    fn extract_from_binary(carrier: &mut T) -> Result<Option<SpanContext>> {
        let baggage_items = Vec::new(); // TODO: Support baggage items

        let mut u64buf: [u8; 8] = [0; 8];
        let mut u8buf: [u8; 1] = [0; 1];

        track!(carrier.read(&mut u64buf[..]).map_err(error::from_io_error))?;
        let trace_id_high = u64::from_be_bytes(u64buf);
        track!(carrier.read(&mut u64buf[..]).map_err(error::from_io_error))?;
        let trace_id_low = u64::from_be_bytes(u64buf);
        track!(carrier.read(&mut u64buf[..]).map_err(error::from_io_error))?;
        let span_id = u64::from_be_bytes(u64buf);
        track!(carrier.read(&mut u64buf[..]).map_err(error::from_io_error))?;
        // parent_span_id attribute is obsolete. Ignore storing it.
        track!(carrier.read(&mut u8buf[..]).map_err(error::from_io_error))?;
        let flags = u8buf[0];

        let state = SpanContextState {
            trace_id: TraceId {
                high: trace_id_high,
                low: trace_id_low,
            },
            span_id,
            flags,
            debug_id: String::new(),
        };
        Ok(Some(SpanContext::new(state, baggage_items)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Tracer;
    use rustracing::sampler::AllSampler;
    use std::collections::HashMap;
    use std::io::Cursor;
    use trackable::error::Failed;
    use trackable::result::TestResult;

    #[test]
    fn trace_id_conversion_works() {
        let id = TraceId { high: 0, low: 10 };
        assert_eq!(id.to_string(), "a");
        assert_eq!("a".parse::<TraceId>().unwrap(), id);

        let id = TraceId { high: 1, low: 2 };
        assert_eq!(id.to_string(), "10000000000000002");
        assert_eq!("10000000000000002".parse::<TraceId>().unwrap(), id);
    }

    #[test]
    fn sampled_flag_works() {
        let state: SpanContextState = "6309ab92c95468edea0dc1a9772ae2dc:409423a204bc17a8:0:1"
            .parse()
            .unwrap();

        assert!(state.is_sampled());
        assert_eq!(state.flags(), 1);

        let state: SpanContextState = "6309ab92c95468edea0dc1a9772ae2dc:409423a204bc17a8:0:0"
            .parse()
            .unwrap();

        assert!(!state.is_sampled());
        assert_eq!(state.flags(), 0);
    }

    #[test]
    fn inject_to_text_map_works() -> TestResult {
        let (span_tx, _span_rx) = crossbeam_channel::bounded(10);
        let tracer = Tracer::with_sender(AllSampler, span_tx);
        let span = tracer.span("test").start();
        let context = track_assert_some!(span.context(), Failed);

        let mut map = HashMap::new();
        track!(context.inject_to_text_map(&mut map))?;
        assert!(map.contains_key(constants::TRACER_CONTEXT_HEADER_NAME));

        Ok(())
    }

    #[test]
    fn extract_from_text_map_works() -> TestResult {
        let mut map = HashMap::new();
        map.insert(
            constants::TRACER_CONTEXT_HEADER_NAME.to_string(),
            "6309ab92c95468edea0dc1a9772ae2dc:409423a204bc17a8:0:1".to_string(),
        );
        let context = track!(SpanContext::extract_from_text_map(&map))?;
        let context = track_assert_some!(context, Failed);
        let trace_id = context.state().trace_id();
        assert_eq!(trace_id.to_string(), "6309ab92c95468edea0dc1a9772ae2dc");

        Ok(())
    }

    /// Official Java client `io.jaegertracing:jaeger-client:0.33.1`
    /// sends HTTP header `uber-trace-id` with url-encoding.
    #[test]
    fn extract_from_urlencoded_text_map_works() -> TestResult {
        let mut map = HashMap::new();
        map.insert(
            constants::TRACER_CONTEXT_HEADER_NAME.to_string(),
            "6309ab92c95468edea0dc1a9772ae2dc%3A409423a204bc17a8%3A0%3A1".to_string(),
        );
        let context = track!(SpanContext::extract_from_text_map(&map))?;
        let context = track_assert_some!(context, Failed);
        let trace_id = context.state().trace_id();
        assert_eq!(trace_id.to_string(), "6309ab92c95468edea0dc1a9772ae2dc");

        Ok(())
    }

    #[test]
    fn extract_debug_id_works() -> TestResult {
        let mut map = HashMap::new();
        map.insert(
            constants::JAEGER_DEBUG_HEADER.to_string(),
            "abcdef".to_string(),
        );
        let context = track!(SpanContext::extract_from_text_map(&map))?;
        let context = track_assert_some!(context, Failed);
        let debug_id = context.state().debug_id();
        assert_eq!(debug_id, Some("abcdef"));

        Ok(())
    }

    #[test]
    fn inject_to_binary_works() -> TestResult {
        let (tracer, _span_rx) = Tracer::new(AllSampler);
        let parent_span = tracer.span("parent_span_test").start();
        let span = tracer
            .span("span_to_be_injected_test")
            .child_of(parent_span.context().unwrap())
            .start();
        let context = track_assert_some!(span.context(), Failed);

        let mut span_buf: Cursor<Vec<u8>> = Cursor::new(vec![]);
        track!(context.clone().inject_to_binary(&mut span_buf))?;

        // deliberately convert io::Cursor<Vec<u8>> to Vec<u8> and re-read elements
        let sbv = span_buf.get_ref().to_vec();
        let mut u64buf: [u8; 8] = [0; 8];
        let mut u32buf: [u8; 4] = [0; 4];
        let mut u8buf: [u8; 1] = [0; 1];

        u64buf.copy_from_slice(&sbv[0..8]);
        assert_eq!(context.state().trace_id().high, u64::from_be_bytes(u64buf));
        u64buf.copy_from_slice(&sbv[8..16]);
        assert_eq!(context.state().trace_id().low, u64::from_be_bytes(u64buf));
        u64buf.copy_from_slice(&sbv[16..24]);
        assert_eq!(context.state().span_id(), u64::from_be_bytes(u64buf));
        u64buf.copy_from_slice(&sbv[24..32]);
        assert_eq!(0, u64::from_be_bytes(u64buf)); // parent_span_id attribute is obsolete.
        u8buf.copy_from_slice(&sbv[32..33]);
        assert_eq!(context.state().flags(), u8buf[0]);
        u32buf.copy_from_slice(&sbv[33..37]);
        assert_eq!(0, u32::from_be_bytes(u32buf)); // no baggage item length

        Ok(())
    }

    #[test]
    fn extract_from_binary_works() -> TestResult {
        let mut span_buf: Cursor<Vec<u8>> = Cursor::new(vec![
            0xab, 0xcd, 0xef, 0xed, 0xcb, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0xbc, 0xde, 0xfe,
            0xdc, 0xba, // trace_id=abcdefedcbabcdef fedcbabcdefedcba
            0, 0, 0, 0, 0, 0, 0, 11, // span_id=11
            0, 0, 0, 0, 0, 0, 0, 12, // parent_span_id=12
            1,  // flags=1
            0, 0, 0, 0, // baggage item length=0
        ]);

        let context = track!(SpanContext::extract_from_binary(&mut span_buf))?;
        let context = track_assert_some!(context, Failed);
        assert_eq!(
            context.state().trace_id().to_string(),
            "abcdefedcbabcdeffedcbabcdefedcba"
        );
        assert_eq!(context.state().span_id(), 11);
        assert_eq!(context.state().flags(), 1);

        // make a span from this context
        let (tracer, _span_rx) = Tracer::new(AllSampler);
        tracer
            .span("test_from_spancontext")
            .child_of(&context)
            .start();

        Ok(())
    }
}
