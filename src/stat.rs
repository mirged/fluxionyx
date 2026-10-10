//! Named modifiers with cached aggregation and optional value bounds.
use crate::{Error, Result, error::finite};
use alloc::{string::String, vec::Vec};

/// Modifier formula: `(base + sum(flat)) * (1 + sum(percent)) * product(factors)`.
/// Percent entries are fractional bonuses (0.1 means +10%).
#[derive(Debug, Clone, PartialEq)]
pub struct Stat {
    base: f64,
    flat: Vec<(String, f64)>,
    percent: Vec<(String, f64)>,
    factors: Vec<(String, f64)>,
    bounds: Option<(f64, f64)>,
    cached: f64,
}
impl Default for Stat {
    fn default() -> Self {
        Self::new(0.0)
    }
}
impl Stat {
    /// Creates a stat. Panics for a nonfinite base; use `try_new` for untrusted input.
    pub fn new(base: f64) -> Self {
        Self::try_new(base).expect("stat base must be finite")
    }
    /// Checked constructor.
    pub fn try_new(base: f64) -> Result<Self> {
        finite(base)?;
        Ok(Self {
            base,
            flat: Vec::new(),
            percent: Vec::new(),
            factors: Vec::new(),
            bounds: None,
            cached: base,
        })
    }
    /// Current cached aggregate.
    pub fn val(&self) -> f64 {
        self.cached
    }
    /// Unmodified base.
    pub fn base(&self) -> f64 {
        self.base
    }
    /// Changes the base without removing modifiers.
    pub fn set_base(&mut self, base: f64) -> Result<()> {
        finite(base)?;
        self.base = base;
        self.recompute();
        Ok(())
    }
    /// Adds or replaces a flat entry in insertion order. Nonfinite values are ignored.
    pub fn add(&mut self, tag: impl AsRef<str>, value: f64) {
        Self::upsert(&mut self.flat, tag.as_ref(), value);
        self.recompute();
    }
    /// Adds or replaces an additive percent bonus. Nonfinite values are ignored.
    pub fn percent(&mut self, tag: impl AsRef<str>, value: f64) {
        Self::upsert(&mut self.percent, tag.as_ref(), value);
        self.recompute();
    }
    /// Adds or replaces an independent factor. Nonfinite values are ignored.
    pub fn mult(&mut self, tag: impl AsRef<str>, value: f64) {
        Self::upsert(&mut self.factors, tag.as_ref(), value);
        self.recompute();
    }
    /// Removes a tag from all modifier layers; returns whether anything changed.
    pub fn remove(&mut self, tag: &str) -> bool {
        let before = self.modifier_count();
        self.flat.retain(|(key, _)| key != tag);
        self.percent.retain(|(key, _)| key != tag);
        self.factors.retain(|(key, _)| key != tag);
        self.recompute();
        before != self.modifier_count()
    }
    /// Number of active entries across all layers.
    pub fn modifier_count(&self) -> usize {
        self.flat.len() + self.percent.len() + self.factors.len()
    }
    /// Removes all modifiers, retaining the base and output bounds.
    pub fn clear(&mut self) {
        self.flat.clear();
        self.percent.clear();
        self.factors.clear();
        self.recompute();
    }
    /// Applies output bounds after aggregation.
    pub fn set_bounds(&mut self, min: f64, max: f64) -> Result<()> {
        finite(min)?;
        finite(max)?;
        if min > max {
            return Err(Error::InvalidBounds);
        }
        self.bounds = Some((min, max));
        self.recompute();
        Ok(())
    }
    /// Removes output bounds.
    pub fn clear_bounds(&mut self) {
        self.bounds = None;
        self.recompute();
    }
    fn upsert(entries: &mut Vec<(String, f64)>, tag: &str, value: f64) {
        if !value.is_finite() {
            return;
        }
        if let Some((_, old)) = entries.iter_mut().find(|(key, _)| key == tag) {
            *old = value;
        } else {
            entries.push((tag.into(), value));
        }
    }
    fn recompute(&mut self) {
        let flat = self.flat.iter().map(|(_, x)| x).sum::<f64>();
        let pct = self.percent.iter().map(|(_, x)| x).sum::<f64>();
        let mult = self.factors.iter().map(|(_, x)| x).product::<f64>();
        let value = (self.base + flat) * (1.0 + pct) * mult;
        self.cached = self
            .bounds
            .map_or(value, |(min, max)| value.clamp(min, max));
    }
}
