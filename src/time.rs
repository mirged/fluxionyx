//! Explicit fixed simulation time, independent from wall clocks.
use crate::{
    Error, Result,
    error::{nonnegative, positive},
};

/// One committed simulation step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    /// One-based tick number.
    pub index: u64,
    /// Simulation time after this step.
    pub elapsed: f64,
    /// Duration in caller-selected time units.
    pub dt: f64,
}
/// A fixed-duration clock. Identical tick counts produce identical times.
#[derive(Debug, Clone, PartialEq)]
pub struct Clock {
    dt: f64,
    ticks: u64,
}
impl Clock {
    /// Creates a clock with a finite positive step duration.
    pub fn new(dt: f64) -> Result<Self> {
        positive(dt)?;
        Ok(Self { dt, ticks: 0 })
    }
    /// Step duration.
    pub fn dt(&self) -> f64 {
        self.dt
    }
    /// Completed tick count.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }
    /// Current time, computed from the tick count.
    pub fn elapsed(&self) -> f64 {
        self.ticks as f64 * self.dt
    }
    /// Commits one tick; failure leaves the clock unchanged.
    pub fn advance(&mut self) -> Result<Step> {
        let next = self.ticks.checked_add(1).ok_or(Error::Overflow)?;
        let elapsed = next as f64 * self.dt;
        if !elapsed.is_finite() {
            return Err(Error::Overflow);
        }
        self.ticks = next;
        Ok(Step {
            index: next,
            elapsed,
            dt: self.dt,
        })
    }
    /// Resets time to zero.
    pub fn reset(&mut self) {
        self.ticks = 0;
    }
}
/// Accumulates frame time and exposes fixed simulation steps on demand.
/// Backlog is retained; the caller chooses how many steps to process each frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedStepper {
    clock: Clock,
    accumulator: f64,
}
impl FixedStepper {
    /// Creates a stepper for the desired simulation interval.
    pub fn new(dt: f64) -> Result<Self> {
        Ok(Self {
            clock: Clock::new(dt)?,
            accumulator: 0.0,
        })
    }
    /// Adds elapsed frame time, rejecting negative, nonfinite, or overflowing values.
    pub fn accumulate(&mut self, dt: f64) -> Result<()> {
        nonnegative(dt)?;
        let next = self.accumulator + dt;
        if !next.is_finite() {
            return Err(Error::Overflow);
        }
        self.accumulator = next;
        Ok(())
    }
    /// Commits a pending step, or returns `None` when less than one interval remains.
    /// A shortfall of up to eight relative machine epsilons is treated as roundoff.
    pub fn next_step(&mut self) -> Result<Option<Step>> {
        let tolerance = 8.0 * f64::EPSILON * self.clock.dt;
        if self.accumulator < self.clock.dt && self.clock.dt - self.accumulator > tolerance {
            return Ok(None);
        }
        let step = self.clock.advance()?;
        self.accumulator = (self.accumulator - self.clock.dt).max(0.0);
        Ok(Some(step))
    }
    /// The simulation clock.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }
    /// Unprocessed frame time.
    pub fn backlog(&self) -> f64 {
        self.accumulator
    }
    /// Fraction of the next interval, clamped while a backlog remains.
    pub fn alpha(&self) -> f64 {
        (self.accumulator / self.clock.dt).clamp(0.0, 1.0)
    }
    /// Resets both simulation time and accumulated frame time.
    pub fn reset(&mut self) {
        self.clock.reset();
        self.accumulator = 0.0;
    }
}
