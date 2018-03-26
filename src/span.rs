//! Span.
//!
//! # References
//!
//! - [constants.go](https://github.com/uber/jaeger-client-go/tree/v2.9.0/constants.go)
//! - [context.go](https://github.com/uber/jaeger-client-go/tree/v2.9.0/context.go)
//! - [propagation.go](https://github.com/uber/jaeger-client-go/tree/v2.9.0/propagation.go)
use std::fmt;
use std::str::{self, FromStr};
use rand;
use rustracing;
use rustracing::carrier::{ExtractFromHttpHeader, InjectToHttpHeader, IterHttpHeaderFields,
                          SetHttpHeaderField};
use rustracing::sampler::BoxSampler;

use {Error, ErrorKind, Result};
use constants;
use error;

/// Span.
pub type Span = rustracing::span::Span<SpanContextState>;

/// Span handle.
pub type SpanHandle = rustracing::span::SpanHandle<SpanContextState>;

/// Finished span.
pub type FinishedSpan = rustracing::span::FinishedSpan<SpanContextState>;

/// Span receiver.
pub type SpanReceiver = rustracing::span::SpanReceiver<SpanContextState>;

/// Options for starting a span.
pub type StartSpanOptions<'a> =
    rustracing::span::StartSpanOptions<'a, BoxSampler<SpanContextState>, SpanContextState>;

/// Candidate span for tracing.
pub type CandidateSpan<'a> = rustracing::span::CandidateSpan<'a, SpanContextState>;

/// Span context.
pub type SpanContext = rustracing::span::SpanContext<SpanContextState>;

/// Span reference.
pub type SpanReference = rustracing::span::SpanReference<SpanContextState>;

const FLAG_SAMPLED: u32 = 0b01;
const FLAG_DEBUG: u32 = 0b10;

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

/// Jager specific span context state.
#[derive(Debug, Clone)]
pub struct SpanContextState {
    trace_id: TraceId,
    span_id: u64,
    flags: u32,
    debug_id: String,
}
impl SpanContextState {
    /// Generate a new SpanContextState with a given TraceId
    pub fn with_trace_id(trace_id: TraceId) -> Self {
        SpanContextState {
            trace_id,
            span_id: rand::random(),
            flags: FLAG_SAMPLED,
            debug_id: String::new(),
        }
    }

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

    pub(crate) fn flags(&self) -> u32 {
        self.flags
    }

    fn root() -> Self {
        Self::with_trace_id(TraceId::default())
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

        macro_rules! token { () => { track_assert_some!(tokens.next(), ErrorKind::InvalidInput) } }
        let trace_id = track!(token!().parse())?;
        let span_id =
            track!(u64::from_str_radix(token!(), 16).map_err(error::from_parse_int_error,))?;
        let _parent_span_id =
            track!(u64::from_str_radix(token!(), 16).map_err(error::from_parse_int_error,))?;
        let flags =
            track!(u32::from_str_radix(token!(), 16).map_err(error::from_parse_int_error,))?;

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
            match name {
                constants::TRACER_CONTEXT_HEADER_NAME => {
                    let value = track!(str::from_utf8(value).map_err(error::from_utf8_error))?;
                    state = Some(track!(value.parse())?);
                }
                constants::JAEGER_DEBUG_HEADER => {
                    let value = track!(str::from_utf8(value).map_err(error::from_utf8_error))?;
                    debug_id = Some(value.to_owned());
                }
                _ => {}
            }
        }
        if let Some(mut state) = state {
            state.flags |= FLAG_SAMPLED;
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn trace_id_conversion_works() {
        let id = TraceId { high: 0, low: 10 };
        assert_eq!(id.to_string(), "a");
        assert_eq!("a".parse::<TraceId>().unwrap(), id);

        let id = TraceId { high: 1, low: 2 };
        assert_eq!(id.to_string(), "10000000000000002");
        assert_eq!("10000000000000002".parse::<TraceId>().unwrap(), id);
    }
}
