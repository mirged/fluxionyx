//! Seeded SplitMix64 randomness for repeatable simulation decisions.
use crate::{
    Error, Result,
    error::{finite, positive},
};

/// Reproducible noncryptographic generator. Same state and calls yield the same
/// integer sequence; transcendental distributions inherit floating-point limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}
impl Rng {
    /// Starts a generator from an explicit seed (zero is valid).
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    /// Current state; recreate with `new(state)` to continue a recorded sequence.
    pub fn state(&self) -> u64 {
        self.state
    }
    /// Next uniform 64-bit word.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    /// Uniform floating-point draw in `[0, 1)` using 53 random bits.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }
    /// Uniform draw in `[min, max)`; equal endpoints return that endpoint.
    pub fn range(&mut self, min: f64, max: f64) -> Result<f64> {
        finite(min)?;
        finite(max)?;
        if min > max {
            return Err(Error::InvalidBounds);
        }
        if min == max {
            return Ok(min);
        }
        let value = crate::curves::lerp(min, max, self.unit());
        // Roundoff can land exactly on the upper endpoint; step down one float.
        Ok(if value >= max {
            if max > 0.0 {
                f64::from_bits(max.to_bits() - 1)
            } else if max == 0.0 {
                -f64::from_bits(1)
            } else {
                f64::from_bits(max.to_bits() + 1)
            }
        } else {
            value.max(min)
        })
    }
    /// Unbiased integer index below `upper`; `None` when the range is empty.
    pub fn index(&mut self, upper: usize) -> Option<usize> {
        if upper == 0 {
            return None;
        }
        let bound = upper as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let n = self.next_u64();
            if n >= threshold {
                return Some((n % bound) as usize);
            }
        }
    }
    /// Bernoulli draw. Probability must be finite and in `[0, 1]`.
    pub fn chance(&mut self, probability: f64) -> Result<bool> {
        finite(probability)?;
        if !(0.0..=1.0).contains(&probability) {
            return Err(Error::OutOfBounds);
        }
        Ok(self.unit() < probability)
    }
    /// In-place Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            values.swap(i, self.index(i + 1).expect("nonempty range"));
        }
    }
    /// Selects an index using nonnegative finite weights; empty/all-zero sets return `None`.
    pub fn weighted_index(&mut self, weights: &[f64]) -> Result<Option<usize>> {
        let mut max: f64 = 0.0;
        for weight in weights {
            crate::error::nonnegative(*weight)?;
            max = max.max(*weight);
        }
        if max == 0.0 {
            return Ok(None);
        }
        let total = weights.iter().map(|weight| weight / max).sum::<f64>();
        let mut draw = self.unit() * total;
        let mut last = None;
        for (index, weight) in weights.iter().enumerate() {
            if *weight > 0.0 {
                last = Some(index);
                let scaled = *weight / max;
                if draw < scaled {
                    return Ok(Some(index));
                }
                draw -= scaled;
            }
        }
        Ok(last)
    }
    /// Exponentially distributed waiting time for a positive event rate.
    pub fn exponential(&mut self, rate: f64) -> Result<f64> {
        positive(rate)?;
        let result = -libm::log1p(-self.unit()) / rate;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(Error::Overflow)
        }
    }
    /// Box-Muller normal draw; finite mean and nonnegative standard deviation required.
    pub fn normal(&mut self, mean: f64, std_dev: f64) -> Result<f64> {
        finite(mean)?;
        crate::error::nonnegative(std_dev)?;
        let radius = libm::sqrt(-2.0 * libm::log1p(-self.unit()));
        let z = radius * libm::cos(core::f64::consts::TAU * self.unit());
        let result = mean + std_dev * z;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(Error::Overflow)
        }
    }
}
