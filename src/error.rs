//! Shared errors and validation for stateful simulation primitives.
use core::fmt;

/// A rejected simulation configuration or operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A number was NaN or infinite.
    NonFinite,
    /// A quantity or rate was negative.
    Negative,
    /// A duration, bracket width, or interval was not positive.
    InvalidDuration,
    /// Minimum exceeded maximum, or their span overflowed.
    InvalidBounds,
    /// Initial state was outside its bounds.
    OutOfBounds,
    /// Dimensions were empty or too large.
    InvalidDimensions,
    /// Recipe coefficients, efficiency, or stock counts were invalid.
    InvalidRecipe,
    /// An event was scheduled before the queue's current time.
    PastEvent,
    /// A counter or computation exceeded its representable range.
    Overflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "value must be finite",
            Self::Negative => "value must be nonnegative",
            Self::InvalidDuration => "duration must be finite and positive",
            Self::InvalidBounds => "bounds must be ordered with a finite span",
            Self::OutOfBounds => "value is outside its bounds",
            Self::InvalidDimensions => "dimensions must be nonzero and fit in memory",
            Self::InvalidRecipe => "invalid recipe coefficients, efficiency, or stock counts",
            Self::PastEvent => "event is before the queue's current time",
            Self::Overflow => "numeric range exceeded",
        })
    }
}
impl core::error::Error for Error {}

/// Result type used by checked configuration and mutation APIs.
pub type Result<T> = core::result::Result<T, Error>;

pub(crate) fn finite(value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::NonFinite)
    }
}
pub(crate) fn nonnegative(value: f64) -> Result<f64> {
    finite(value)?;
    if value >= 0.0 {
        Ok(value)
    } else {
        Err(Error::Negative)
    }
}
pub(crate) fn positive(value: f64) -> Result<f64> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(Error::InvalidDuration)
    }
}
