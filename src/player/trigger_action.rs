#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerAction {
    Toggle,
    Next,
    Previous,
    StepBack,
    StepForward,
    StopOngoing,
}
