use crate::domain::runtime::value_objects::sync_offset_ms::SyncOffsetMs;
use crate::domain::shared::value_object::ValueObject;

#[derive(Debug, Clone, PartialEq)]
pub struct SyncProfile {
    pub key: String,
    pub offset_ms: SyncOffsetMs,
}

impl ValueObject for SyncProfile {}
