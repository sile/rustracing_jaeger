//! Thrift components defined in [jaeger.thrift].
//!
//! [jaeger.thrift]: https://github.com/uber/jaeger-idl/blob/master/thrift/jaeger.thrift
use crate::constants;
use crate::span::{FinishedSpan, SpanReference};
use std::time::{SystemTime, UNIX_EPOCH};
use thrift_codec::data::{Field, List, Struct};

/// `TagKind` denotes the kind of a `Tag`'s value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
pub enum TagKind {
    String = 0,
    Double = 1,
    Bool = 2,
    Long = 3,
    Binary = 4,
}

/// `Tag` is a basic strongly typed key/value pair.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[allow(missing_docs)]
pub enum Tag {
    String { key: String, value: String },
    Double { key: String, value: f64 },
    Bool { key: String, value: bool },
    Long { key: String, value: i64 },
    Binary { key: String, value: Vec<u8> },
}
impl Tag {
    /// Returns the key of this tag.
    pub fn key(&self) -> &str {
        match *self {
            Tag::String { ref key, .. }
            | Tag::Double { ref key, .. }
            | Tag::Bool { ref key, .. }
            | Tag::Long { ref key, .. }
            | Tag::Binary { ref key, .. } => key,
        }
    }

    /// Returns the kind of this tag.
    pub fn kind(&self) -> TagKind {
        match *self {
            Tag::String { .. } => TagKind::String,
            Tag::Double { .. } => TagKind::Double,
            Tag::Bool { .. } => TagKind::Bool,
            Tag::Long { .. } => TagKind::Long,
            Tag::Binary { .. } => TagKind::Binary,
        }
    }
}
impl From<Tag> for Struct {
    fn from(f: Tag) -> Self {
        let mut fields = vec![Field::new(1, f.key()), Field::new(2, f.kind() as i32)];
        match f {
            Tag::String { value, .. } => fields.push(Field::new(3, value)),
            Tag::Double { value, .. } => fields.push(Field::new(4, value)),
            Tag::Bool { value, .. } => fields.push(Field::new(5, value)),
            Tag::Long { value, .. } => fields.push(Field::new(6, value)),
            Tag::Binary { value, .. } => fields.push(Field::new(7, value)),
        };
        Struct::new(fields)
    }
}
impl<'a> From<&'a rustracing::tag::Tag> for Tag {
    fn from(f: &'a rustracing::tag::Tag) -> Self {
        use rustracing::tag::TagValue;
        let key = f.name().to_owned();
        match *f.value() {
            TagValue::Boolean(value) => Tag::Bool { key, value },
            TagValue::Float(value) => Tag::Double { key, value },
            TagValue::Integer(value) => Tag::Long { key, value },
            TagValue::String(ref value) => Tag::String {
                key,
                value: value.as_ref().to_owned(),
            },
        }
    }
}
impl<'a> From<&'a rustracing::log::LogField> for Tag {
    fn from(f: &'a rustracing::log::LogField) -> Self {
        Tag::String {
            key: f.name().to_owned(),
            value: f.value().to_owned(),
        }
    }
}

/// `Log` is a timed even with an arbitrary set of tags.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct Log {
    pub timestamp: i64,
    pub fields: Vec<Tag>,
}
impl From<Log> for Struct {
    fn from(f: Log) -> Self {
        Struct::from((
            f.timestamp,
            List::from(f.fields.into_iter().map(Struct::from).collect::<Vec<_>>()),
        ))
    }
}
impl<'a> From<&'a rustracing::log::Log> for Log {
    fn from(f: &'a rustracing::log::Log) -> Self {
        Log {
            timestamp: elapsed(UNIX_EPOCH, f.time()),
            fields: f.fields().iter().map(From::from).collect(),
        }
    }
}

/// Span reference kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
pub enum SpanRefKind {
    ChildOf = 0,
    FollowsFrom = 1,
}

/// `SpanRef` describes causal relationship of the current span to another span (e.g. 'child-of')
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct SpanRef {
    pub kind: SpanRefKind,
    pub trace_id_low: i64,
    pub trace_id_high: i64,
    pub span_id: i64,
}
impl From<SpanRef> for Struct {
    fn from(f: SpanRef) -> Self {
        Struct::from((f.kind as i32, f.trace_id_low, f.trace_id_high, f.span_id))
    }
}
impl<'a> From<&'a SpanReference> for SpanRef {
    fn from(f: &'a SpanReference) -> Self {
        let kind = if f.is_child_of() {
            SpanRefKind::ChildOf
        } else {
            SpanRefKind::FollowsFrom
        };
        SpanRef {
            kind,
            trace_id_low: f.span().trace_id().low as i64,
            trace_id_high: f.span().trace_id().high as i64,
            span_id: f.span().span_id() as i64,
        }
    }
}

/// `Span` represents a named unit of work performed by a service.
#[derive(Debug, Clone)]
pub struct Span {
    /// The least significant 64 bits of a traceID.
    pub trace_id_low: i64,

    /// The most significant 64 bits of a traceID; 0 when only 64bit IDs are used.
    pub trace_id_high: i64,

    /// Unique span id (only unique within a given trace).
    pub span_id: i64,

    /// Since nearly all spans will have parents spans, `ChildOf` refs do not have to be explicit.
    ///
    /// Should be `0` if the current span is a root span.
    pub parent_span_id: i64,

    /// The name of operation.
    pub operation_name: String,

    /// Causal references to other spans.
    pub references: Vec<SpanRef>,

    /// A bit field used to propagate sampling decisions.
    ///
    /// `1` signifies a SAMPLED span, `2` signifies a DEBUG span.
    pub flags: i32,

    /// Start time of this span.
    pub start_time: i64,

    /// Duration of this span.
    pub duration: i64,

    /// Tag list.
    pub tags: Vec<Tag>,

    /// Log list.
    pub logs: Vec<Log>,
}
impl From<Span> for Struct {
    fn from(f: Span) -> Self {
        let mut fields = Vec::with_capacity(11);
        fields.push(Field::new(1, f.trace_id_low));
        fields.push(Field::new(2, f.trace_id_high));
        fields.push(Field::new(3, f.span_id));
        fields.push(Field::new(4, f.parent_span_id));
        fields.push(Field::new(5, f.operation_name));
        if !f.references.is_empty() {
            fields.push(Field::new(
                6,
                List::from(
                    f.references
                        .into_iter()
                        .map(Struct::from)
                        .collect::<Vec<_>>(),
                ),
            ));
        }
        fields.push(Field::new(7, f.flags));
        fields.push(Field::new(8, f.start_time));
        fields.push(Field::new(9, f.duration));
        if !f.tags.is_empty() {
            fields.push(Field::new(
                10,
                List::from(f.tags.into_iter().map(Struct::from).collect::<Vec<_>>()),
            ));
        }
        if !f.logs.is_empty() {
            fields.push(Field::new(
                11,
                List::from(f.logs.into_iter().map(Struct::from).collect::<Vec<_>>()),
            ));
        }
        Struct::new(fields)
    }
}
impl<'a> From<&'a FinishedSpan> for Span {
    fn from(f: &'a FinishedSpan) -> Self {
        let state = f.context().state();
        let parent_span_id = f
            .references()
            .iter()
            .find(|r| r.span().is_sampled())
            .map(|r| r.span().span_id() as i64)
            .unwrap_or(0);
        let mut span = Span {
            trace_id_low: state.trace_id().low as i64,
            trace_id_high: state.trace_id().high as i64,
            span_id: state.span_id() as i64,
            parent_span_id,
            operation_name: f.operation_name().to_owned(),
            references: f
                .references()
                .iter()
                .filter(|r| r.span().is_sampled())
                .map(From::from)
                .collect(),
            flags: state.flags() as i32,
            start_time: elapsed(UNIX_EPOCH, f.start_time()),
            duration: elapsed(f.start_time(), f.finish_time()),
            tags: f.tags().iter().map(From::from).collect(),
            logs: f.logs().iter().map(From::from).collect(),
        };
        if let Some(id) = state.debug_id() {
            span.tags.push(Tag::from(&rustracing::tag::Tag::new(
                constants::JAEGER_DEBUG_HEADER,
                id.to_owned(),
            )));
        }
        span
    }
}

fn elapsed(start: SystemTime, finish: SystemTime) -> i64 {
    if let Ok(d) = finish.duration_since(start) {
        (d.as_secs() * 1_000_000 + u64::from(d.subsec_nanos()) / 1000) as i64
    } else {
        let d = start.duration_since(finish).expect("Never fails");
        -((d.as_secs() * 1_000_000 + u64::from(d.subsec_nanos()) / 1000) as i64)
    }
}

/// `Process` describes the traced process/service that emits spans.
#[derive(Debug, Clone)]
pub struct Process {
    /// The name of this service.
    pub service_name: String,

    /// Tag list.
    pub tags: Vec<Tag>,
}
impl From<Process> for Struct {
    fn from(f: Process) -> Self {
        let tags = List::from(f.tags.into_iter().map(Struct::from).collect::<Vec<_>>());
        if tags.is_empty() {
            Struct::from((f.service_name,))
        } else {
            Struct::from((f.service_name, tags))
        }
    }
}

/// `Batch` is a collection of spans reported out of process.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct Batch {
    pub process: Process,
    pub spans: Vec<Span>,
}
impl From<Batch> for Struct {
    fn from(f: Batch) -> Self {
        Struct::from((
            Struct::from(f.process),
            List::from(f.spans.into_iter().map(Struct::from).collect::<Vec<_>>()),
        ))
    }
}
