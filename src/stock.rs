//! Bounded resources, explicit rates, and observable integration.
use crate::{
    Error, Result,
    error::{finite, nonnegative, positive},
};

/// The requested and applied changes from one Euler integration step.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TickReport {
    /// Change implied by the net rate and elapsed time.
    pub requested: f64,
    /// Change applied after boundary constraints.
    pub applied: f64,
    /// Requested change minus applied change (signed).
    pub clipped: f64,
}

/// A scalar resource whose private state always respects its bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct Stock {
    current: f64,
    min: f64,
    max: Option<f64>,
    inflow_rate: f64,
    outflow_rate: f64,
}
impl Default for Stock {
    fn default() -> Self {
        Self {
            current: 0.0,
            min: 0.0,
            max: None,
            inflow_rate: 0.0,
            outflow_rate: 0.0,
        }
    }
}
impl Stock {
    /// Creates a nonnegative resource with no upper bound.
    pub fn new(initial: f64) -> Result<Self> {
        nonnegative(initial)?;
        Ok(Self {
            current: initial,
            ..Self::default()
        })
    }
    /// Creates a resource in `[min, max]`; bounds may be negative.
    pub fn bounded(min: f64, max: f64, initial: f64) -> Result<Self> {
        finite(min)?;
        finite(max)?;
        finite(initial)?;
        if min > max || !(max - min).is_finite() {
            return Err(Error::InvalidBounds);
        }
        if initial < min || initial > max {
            return Err(Error::OutOfBounds);
        }
        Ok(Self {
            current: initial,
            min,
            max: Some(max),
            ..Self::default()
        })
    }
    /// Current stored quantity.
    pub fn val(&self) -> f64 {
        self.current
    }
    /// Lower bound.
    pub fn min(&self) -> f64 {
        self.min
    }
    /// Optional upper bound.
    pub fn max(&self) -> Option<f64> {
        self.max
    }
    /// Quantity that can be withdrawn without crossing the lower bound.
    pub fn available(&self) -> f64 {
        self.current - self.min
    }
    /// Remaining capacity, or infinity for an unbounded stock.
    pub fn room(&self) -> f64 {
        self.max.map_or(f64::INFINITY, |max| max - self.current)
    }
    pub(crate) fn receivable(&self) -> f64 {
        self.room().min(f64::MAX - self.current)
    }
    /// Fill ratio relative to the bounds; a zero-width stock is full.
    pub fn ratio(&self) -> Option<f64> {
        self.max.map(|max| {
            if max == self.min {
                1.0
            } else {
                (self.current - self.min) / (max - self.min)
            }
        })
    }
    /// Whether there is no withdrawable quantity.
    pub fn is_empty(&self) -> bool {
        self.current <= self.min
    }
    /// Whether capacity has been reached.
    pub fn is_full(&self) -> bool {
        self.max.is_some_and(|max| self.current >= max)
    }
    /// Replaces the value, rejecting invalid values without changing state.
    pub fn set(&mut self, value: f64) -> Result<()> {
        finite(value)?;
        if value < self.min || self.max.is_some_and(|max| value > max) {
            return Err(Error::OutOfBounds);
        }
        self.current = value;
        Ok(())
    }
    /// Deposits up to the remaining capacity and returns actual accepted quantity.
    /// Nonpositive or nonfinite requests are ignored; overflow is clipped to f64::MAX.
    pub fn deposit(&mut self, amount: f64) -> f64 {
        if !amount.is_finite() || amount <= 0.0 {
            return 0.0;
        }
        let old = self.current;
        self.current = (old + amount.min(self.receivable())).min(self.max.unwrap_or(f64::MAX));
        self.current - old
    }
    /// Withdraws up to the available quantity and returns actual removal.
    pub fn withdraw(&mut self, amount: f64) -> f64 {
        if !amount.is_finite() || amount <= 0.0 {
            return 0.0;
        }
        let old = self.current;
        self.current = (old - amount.min(self.available())).max(self.min);
        old - self.current
    }
    /// Accumulates a finite positive inflow rate. Invalid/overflowing rates are ignored.
    pub fn inflow(&mut self, rate: f64) {
        if rate > 0.0 && (self.inflow_rate + rate).is_finite() {
            self.inflow_rate += rate;
        }
    }
    /// Accumulates a finite positive outflow rate. Invalid/overflowing rates are ignored.
    pub fn outflow(&mut self, rate: f64) {
        if rate > 0.0 && (self.outflow_rate + rate).is_finite() {
            self.outflow_rate += rate;
        }
    }
    /// Returns `(inflow, outflow)` rates awaiting integration.
    pub fn rates(&self) -> (f64, f64) {
        (self.inflow_rate, self.outflow_rate)
    }
    /// Discards pending rates.
    pub fn clear_rates(&mut self) {
        self.inflow_rate = 0.0;
        self.outflow_rate = 0.0;
    }
    /// Integrates finite rates and clears them on success; errors leave all state unchanged.
    pub fn integrate(&mut self, dt: f64) -> Result<TickReport> {
        positive(dt)?;
        let requested = (self.inflow_rate - self.outflow_rate) * dt;
        finite(requested).map_err(|_| Error::Overflow)?;
        let next = self.current + requested;
        if !next.is_finite() {
            return Err(Error::Overflow);
        }
        let old = self.current;
        self.current = next.max(self.min).min(self.max.unwrap_or(f64::MAX));
        self.clear_rates();
        let applied = self.current - old;
        Ok(TickReport {
            requested,
            applied,
            clipped: requested - applied,
        })
    }
    /// Convenience integration; invalid steps leave the value and pending rates unchanged.
    pub fn tick(&mut self, dt: f64) {
        let _ = self.integrate(dt);
    }
    /// Conserved transfer constrained by both stocks' bounds.
    pub fn flow_to(&mut self, target: &mut Self, amount: f64) -> f64 {
        crate::flow::flow(self, target, amount)
    }
    /// Linear loss, in units per time.
    pub fn leak(&mut self, rate: f64, dt: f64) -> f64 {
        crate::flow::leak(self, rate, dt)
    }
    /// Exponential loss of quantity above the lower bound, parameterized by half-life.
    pub fn evaporate(&mut self, half_life: f64, dt: f64) -> f64 {
        crate::flow::evaporate(self, half_life, dt)
    }
}
