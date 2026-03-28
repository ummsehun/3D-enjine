#[path = "app_impl.rs"]
mod app_impl;

pub use self::app_impl::run;
pub(crate) use self::app_impl::{persist_sync_profile_offset, set_runtime_panic_state};
