//! Platform-independent logic for tally: activities, sampling, storage and reports.

pub mod activity;
pub mod duration;
pub mod error;
pub mod export;
pub mod range;
pub mod scheduler;
pub mod settings;
pub mod store;

pub use activity::ActivityName;
pub use error::{Error, Result};
