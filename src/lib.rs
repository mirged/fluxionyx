#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

extern crate alloc;

pub mod control;
pub mod curves;
pub mod error;
#[cfg(feature = "fixed")]
pub mod fixed;
pub mod flow;
pub mod population;
pub mod random;
pub mod recipe;
pub mod schedule;
pub mod spatial;
pub mod stat;
pub mod statistics;
pub mod stock;
pub mod time;

pub use error::{Error, Result};
pub use population::{AgeDistribution, Cohort};
pub use spatial::{Boundary, Field2D};
pub use stat::Stat;
pub use stock::Stock;

/// Common simulation building blocks in a single import.
pub mod prelude {
    pub use crate::control::{Delay, Smoother, Threshold};
    pub use crate::curves::*;
    pub use crate::flow::*;
    pub use crate::population::{AgeDistribution, Cohort};
    pub use crate::random::Rng;
    pub use crate::recipe::{Recipe, RecipeReport, convert, convert_lossy};
    pub use crate::schedule::{EventId, EventQueue, Scheduled};
    pub use crate::spatial::{Boundary, Field2D};
    pub use crate::stat::Stat;
    pub use crate::statistics::{RunningStats, Window};
    pub use crate::stock::{Stock, TickReport};
    pub use crate::time::{Clock, FixedStepper, Step};
    pub use crate::{Error, Result};
}
