use rand;
use rustracing;
use rustracing::span::CandidateSpan;

pub type Span = rustracing::span::Span<SpanContext>;
pub type FinishedSpan = rustracing::span::FinishedSpan<SpanContext>;

#[derive(Debug, Clone)]
pub struct SpanContext {
    trace_id: [u8; 16],
    span_id: u64,
    flags: u32,
}
impl SpanContext {
    pub fn root() -> Self {
        Self::with_trace_id(rand::random())
    }
    pub fn with_trace_id(trace_id: [u8; 16]) -> Self {
        SpanContext {
            trace_id,
            span_id: rand::random(),
            flags: 0,
        }
    }
    pub fn trace_id(&self) -> [u8; 16] {
        self.trace_id
    }
    pub fn span_id(&self) -> u64 {
        self.span_id
    }
    pub fn flags(&self) -> u32 {
        self.flags
    }
}
impl<'a> From<CandidateSpan<'a, SpanContext>> for SpanContext {
    fn from(f: CandidateSpan<'a, Self>) -> Self {
        if let Some(primary) = f.references().first() {
            Self::with_trace_id(primary.trace_id)
        } else {
            Self::root()
        }
    }
}
