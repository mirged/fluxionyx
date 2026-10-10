//! Feedback helpers: exponential smoothing, hysteresis, and tick-based delays.
use crate::{
    Error, Result,
    error::{finite, nonnegative, positive},
};
use alloc::collections::VecDeque;

/// Exponential smoothing with frame-rate-independent half-life semantics.
#[derive(Debug, Clone, PartialEq)]
pub struct Smoother {
    value: f64,
    half_life: f64,
}
impl Smoother {
    /// Constructs with a finite value and positive half-life.
    pub fn new(value: f64, half_life: f64) -> Result<Self> {
        finite(value)?;
        positive(half_life)?;
        Ok(Self { value, half_life })
    }
    /// Current smoothed value.
    pub fn val(&self) -> f64 {
        self.value
    }
    /// Approaches a target over an explicit finite nonnegative duration.
    pub fn update(&mut self, target: f64, dt: f64) -> Result<f64> {
        finite(target)?;
        nonnegative(dt)?;
        self.value = crate::curves::approach(self.value, target, self.half_life, dt);
        Ok(self.value)
    }
    /// Immediately replaces the current value.
    pub fn reset(&mut self, value: f64) -> Result<()> {
        finite(value)?;
        self.value = value;
        Ok(())
    }
}
/// Two-threshold switch. Activation occurs at `on`, deactivation at `off`.
/// The interval between thresholds retains the previous state and avoids chatter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threshold {
    off: f64,
    on: f64,
    active: bool,
}
impl Threshold {
    /// Creates an initially inactive threshold with `off < on`.
    pub fn new(off: f64, on: f64) -> Result<Self> {
        finite(off)?;
        finite(on)?;
        if off >= on {
            return Err(Error::InvalidBounds);
        }
        Ok(Self {
            off,
            on,
            active: false,
        })
    }
    /// Current switch state.
    pub fn active(&self) -> bool {
        self.active
    }
    /// Applies a finite observation and returns the resulting state.
    pub fn update(&mut self, value: f64) -> Result<bool> {
        finite(value)?;
        if value >= self.on {
            self.active = true;
        } else if value <= self.off {
            self.active = false;
        }
        Ok(self.active)
    }
    /// Explicitly sets the retained state.
    pub fn reset(&mut self, active: bool) {
        self.active = active;
    }
}
/// A delay line of exactly `ticks` pushes, initialized with copies of one value.
#[derive(Debug, Clone, PartialEq)]
pub struct Delay<T> {
    queue: VecDeque<T>,
}
impl<T: Clone> Delay<T> {
    /// Builds a delay. Zero ticks passes observations through immediately.
    pub fn new(ticks: usize, initial: T) -> Self {
        Self {
            queue: core::iter::repeat_n(initial, ticks).collect(),
        }
    }
    /// Pushes the next value and returns the value from the delayed tick.
    pub fn push(&mut self, value: T) -> T {
        if let Some(previous) = self.queue.pop_front() {
            self.queue.push_back(value);
            previous
        } else {
            value
        }
    }
    /// Number of delayed ticks.
    pub fn ticks(&self) -> usize {
        self.queue.len()
    }
    /// Replaces all retained observations.
    pub fn reset(&mut self, value: T) {
        for slot in &mut self.queue {
            *slot = value.clone();
        }
    }
}
