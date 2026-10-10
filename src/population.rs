//! Population brackets, migration, mortality, and whole/fractional ageing.
use crate::{
    Error, Result,
    error::{nonnegative, positive},
};
use alloc::{vec, vec::Vec};

/// Counts in equal-width age brackets, assumed uniform within each bracket.
#[derive(Debug, Clone, PartialEq)]
pub struct Cohort {
    buckets: Vec<f64>,
    scratch: Vec<f64>,
    bucket_years: f64,
}
/// Alternative name for a population pyramid.
pub type AgeDistribution = Cohort;

impl Cohort {
    /// Constructs empty brackets. Panics for invalid dimensions/width.
    pub fn new(num_brackets: usize, bucket_years: f64) -> Self {
        Self::try_new(num_brackets, bucket_years).expect("invalid cohort dimensions")
    }
    /// Checked empty constructor.
    pub fn try_new(num_brackets: usize, bucket_years: f64) -> Result<Self> {
        positive(bucket_years)?;
        if num_brackets == 0
            || num_brackets > isize::MAX as usize / core::mem::size_of::<f64>()
            || !(num_brackets as f64 * bucket_years).is_finite()
        {
            return Err(Error::InvalidDimensions);
        }
        Ok(Self {
            buckets: vec![0.0; num_brackets],
            scratch: vec![0.0; num_brackets],
            bucket_years,
        })
    }
    /// Seeds a pyramid directly; all counts must be nonnegative and finite.
    pub fn from_buckets(counts: &[f64], bucket_years: f64) -> Result<Self> {
        for count in counts {
            nonnegative(*count)?;
        }
        if !counts.iter().sum::<f64>().is_finite() {
            return Err(Error::Overflow);
        }
        let mut cohort = Self::try_new(counts.len(), bucket_years)?;
        cohort.buckets.copy_from_slice(counts);
        Ok(cohort)
    }
    /// Immutable bucket counts.
    pub fn buckets(&self) -> &[f64] {
        &self.buckets
    }
    /// Age range covered by each bracket.
    pub fn bucket_years(&self) -> f64 {
        self.bucket_years
    }
    /// Maximum represented age (exclusive).
    pub fn max_age(&self) -> f64 {
        self.buckets.len() as f64 * self.bucket_years
    }
    /// Adds newborns. Invalid or overflowing requests are ignored.
    pub fn births(&mut self, count: f64) {
        let _ = self.migrate(0, count);
    }
    /// Adds or removes individuals at a bucket; removals stop at zero.
    /// Returns the actual signed population change.
    pub fn migrate(&mut self, bucket: usize, amount: f64) -> Result<f64> {
        crate::error::finite(amount)?;
        if bucket >= self.buckets.len() {
            return Err(Error::OutOfBounds);
        }
        if amount > 0.0 && !(self.total() + amount).is_finite() {
            return Err(Error::Overflow);
        }
        let old = self.buckets[bucket];
        self.buckets[bucket] = (old + amount).max(0.0);
        Ok(self.buckets[bucket] - old)
    }
    /// Linear age-dependent mortality. The curve receives bracket start age in years.
    /// Nonfinite or negative rates and invalid durations are ignored.
    pub fn cull_rate<F: Fn(usize) -> f64>(&mut self, curve: F, dt: f64) -> f64 {
        if !dt.is_finite() || dt <= 0.0 {
            return 0.0;
        }
        let mut deaths = 0.0;
        for (i, count) in self.buckets.iter_mut().enumerate() {
            let rate = curve((i as f64 * self.bucket_years) as usize);
            if rate.is_finite() && rate > 0.0 {
                let lost = *count * (rate * dt).min(1.0);
                *count -= lost;
                deaths += lost;
            }
        }
        deaths
    }
    /// Continuous hazard mortality, using survival `exp(-rate * dt)`.
    /// The curve receives bracket start age as a floating-point year value.
    pub fn cull_hazard<F: Fn(f64) -> f64>(&mut self, curve: F, dt: f64) -> f64 {
        if !dt.is_finite() || dt <= 0.0 {
            return 0.0;
        }
        let mut deaths = 0.0;
        for (i, count) in self.buckets.iter_mut().enumerate() {
            let rate = curve(i as f64 * self.bucket_years);
            if rate.is_finite() && rate > 0.0 {
                let lost = *count * -libm::expm1(-rate * dt);
                *count -= lost;
                deaths += lost;
            }
        }
        deaths
    }
    /// Ages by any finite positive duration, including multiple bracket widths.
    /// Returns individuals who moved beyond the represented age horizon.
    /// Fractional shifts interpolate assuming uniform bracket populations.
    pub fn advance(&mut self, dt_years: f64) -> f64 {
        if !dt_years.is_finite() || dt_years <= 0.0 {
            return 0.0;
        }
        let total = self.total();
        let shift = dt_years / self.bucket_years;
        self.scratch.fill(0.0);
        if shift < self.buckets.len() as f64 {
            let whole = libm::floor(shift) as usize;
            let fraction = shift - whole as f64;
            for (i, count) in self.buckets.iter().enumerate() {
                let dest = i + whole;
                if dest < self.scratch.len() {
                    self.scratch[dest] += count * (1.0 - fraction);
                }
                if dest + 1 < self.scratch.len() {
                    self.scratch[dest + 1] += count * fraction;
                }
            }
        }
        core::mem::swap(&mut self.buckets, &mut self.scratch);
        (total - self.total()).max(0.0)
    }
    /// Total population.
    pub fn total(&self) -> f64 {
        self.buckets.iter().sum()
    }
    /// Population in `[min_age, max_age)`, weighting partial bracket overlap.
    pub fn working_slice(&self, min_age: f64, max_age: f64) -> f64 {
        if !min_age.is_finite() || !max_age.is_finite() || min_age >= max_age {
            return 0.0;
        }
        self.buckets
            .iter()
            .enumerate()
            .map(|(i, count)| {
                let start = i as f64 * self.bucket_years;
                let overlap =
                    ((start + self.bucket_years).min(max_age) - start.max(min_age)).max(0.0);
                count * (overlap / self.bucket_years)
            })
            .sum()
    }
    /// Approximate mean age using bracket midpoints; `None` for empty populations.
    pub fn mean_age(&self) -> Option<f64> {
        let total = self.total();
        if total == 0.0 {
            return None;
        }
        Some(
            self.buckets
                .iter()
                .enumerate()
                .map(|(i, count)| (count / total) * (i as f64 + 0.5) * self.bucket_years)
                .sum(),
        )
    }
}
