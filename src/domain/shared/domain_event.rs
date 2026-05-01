use std::fmt::Debug;

pub trait DomainEvent: Debug + Send + Sync {
    fn event_type(&self) -> &'static str;
}
