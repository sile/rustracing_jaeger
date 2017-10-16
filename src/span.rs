use rand;
use rustracing;
use rustracing::sampler::BoxSampler;

pub type Span = rustracing::span::Span<SpanContextState>;
pub type FinishedSpan = rustracing::span::FinishedSpan<SpanContextState>;
pub type SpanReceiver = rustracing::span::SpanReceiver<SpanContextState>;
pub type StartSpanOptions<'a> = rustracing::span::StartSpanOptions<
    'a,
    BoxSampler<SpanContextState>,
    SpanContextState,
>;
pub type CandidateSpan<'a> = rustracing::span::CandidateSpan<'a, SpanContextState>;
pub type SpanContext = rustracing::span::SpanContext<SpanContextState>;
pub type SpanReference = rustracing::span::SpanReference<SpanContextState>;

#[derive(Debug, Clone)]
pub struct SpanContextState {
    trace_id: [u8; 16],
    span_id: u64,
    flags: u32,
}
impl SpanContextState {
    pub fn root() -> Self {
        Self::with_trace_id(rand::random())
    }
    pub fn with_trace_id(trace_id: [u8; 16]) -> Self {
        SpanContextState {
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
impl<'a> From<CandidateSpan<'a>> for SpanContextState {
    fn from(f: CandidateSpan<'a>) -> Self {
        if let Some(primary) = f.references().first() {
            Self::with_trace_id(primary.trace_id)
        } else {
            Self::root()
        }
    }
}
