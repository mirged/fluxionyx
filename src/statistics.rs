//! Online observations and bounded rolling histories.
use crate::{Error, Result, error::finite};
use alloc::collections::VecDeque;

/// Welford online mean/variance with constant memory.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RunningStats {
    count: u64,
    mean: f64,
    m2: f64,
    min: f64,
    max: f64,
}
impl RunningStats {
    /// An empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }
    /// Number of accepted observations.
    pub fn count(&self) -> u64 {
        self.count
    }
    /// Mean, or `None` before the first observation.
    pub fn mean(&self) -> Option<f64> {
        (self.count > 0).then_some(self.mean)
    }
    /// Population variance, or `None` when empty.
    pub fn variance(&self) -> Option<f64> {
        (self.count > 0).then(|| (self.m2 / self.count as f64).max(0.0))
    }
    /// Sample variance, requiring at least two observations.
    pub fn sample_variance(&self) -> Option<f64> {
        (self.count > 1).then(|| (self.m2 / (self.count - 1) as f64).max(0.0))
    }
    /// Population standard deviation.
    pub fn std_dev(&self) -> Option<f64> {
        self.variance().map(libm::sqrt)
    }
    /// Observed minimum and maximum.
    pub fn range(&self) -> Option<(f64, f64)> {
        (self.count > 0).then_some((self.min, self.max))
    }
    /// Adds an observation; nonfinite values or overflow leave the accumulator unchanged.
    pub fn push(&mut self, value: f64) -> Result<()> {
        finite(value)?;
        if self.count == 0 {
            *self = Self {
                count: 1,
                mean: value,
                m2: 0.0,
                min: value,
                max: value,
            };
            return Ok(());
        }
        let count = self.count.checked_add(1).ok_or(Error::Overflow)?;
        let delta = value - self.mean;
        let mean = self.mean + delta / count as f64;
        let m2 = self.m2 + delta * (value - mean);
        if !mean.is_finite() || !m2.is_finite() {
            return Err(Error::Overflow);
        }
        *self = Self {
            count,
            mean,
            m2,
            min: self.min.min(value),
            max: self.max.max(value),
        };
        Ok(())
    }
    /// Combines independent accumulators without replaying their observations.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if other.count == 0 {
            return Ok(());
        }
        if self.count == 0 {
            *self = *other;
            return Ok(());
        }
        let count = self.count.checked_add(other.count).ok_or(Error::Overflow)?;
        let delta = other.mean - self.mean;
        let weight = other.count as f64 / count as f64;
        let mean = self.mean + delta * weight;
        let m2 = self.m2 + other.m2 + delta * delta * (self.count as f64 * weight);
        if !mean.is_finite() || !m2.is_finite() {
            return Err(Error::Overflow);
        }
        *self = Self {
            count,
            mean,
            m2,
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        };
        Ok(())
    }
    /// Clears observations.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
/// Fixed-capacity history. Allocates once; updates reuse storage.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    values: VecDeque<f64>,
    capacity: usize,
}
impl Window {
    /// Creates a nonempty rolling window.
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > isize::MAX as usize / core::mem::size_of::<f64>() {
            return Err(Error::InvalidDimensions);
        }
        Ok(Self {
            values: VecDeque::with_capacity(capacity),
            capacity,
        })
    }
    /// Adds an observation; returns the evicted oldest value, if any.
    pub fn push(&mut self, value: f64) -> Result<Option<f64>> {
        finite(value)?;
        let evicted = if self.values.len() == self.capacity {
            self.values.pop_front()
        } else {
            None
        };
        self.values.push_back(value);
        Ok(evicted)
    }
    /// Number of stored observations.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Whether there are no observations.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Maximum stored observation count.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    /// Observations from oldest to newest.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &f64> {
        self.values.iter()
    }
    /// Mean of retained observations; no accumulated rolling-sum drift.
    pub fn mean(&self) -> Option<f64> {
        if self.is_empty() {
            None
        } else {
            Some(self.values.iter().map(|x| x / self.len() as f64).sum())
        }
    }
    /// Clears retained observations without freeing storage.
    pub fn clear(&mut self) {
        self.values.clear();
    }
}
