//! Validated reusable stoichiometric recipes with dry-run planning.
use crate::{Error, Result, Stock, error::nonnegative};
use alloc::vec::Vec;

/// Production outcome; amounts are totals across all inputs and outputs.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RecipeReport {
    /// Number of batches requested.
    pub requested: f64,
    /// Number of batches possible/completed.
    pub completed: f64,
    /// Sum of input quantities used; meaningful only for compatible units.
    pub consumed: f64,
    /// Sum of effective output quantities; meaningful only for compatible units.
    pub produced: f64,
}
/// Immutable coefficients reusable against different sets of stocks.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    inputs: Vec<f64>,
    outputs: Vec<f64>,
    efficiency: f64,
}

impl Recipe {
    /// Constructs a recipe. Coefficients must be nonnegative and finite, with
    /// at least one positive coefficient on each side. Efficiency is in `[0, 1]`.
    pub fn new(inputs: &[f64], outputs: &[f64], efficiency: f64) -> Result<Self> {
        validate(inputs.iter().copied(), outputs.iter().copied(), efficiency)?;
        Ok(Self {
            inputs: inputs.into(),
            outputs: outputs.into(),
            efficiency,
        })
    }
    /// Input quantities per batch, in binding order.
    pub fn inputs(&self) -> &[f64] {
        &self.inputs
    }
    /// Output quantities per batch before efficiency, in binding order.
    pub fn outputs(&self) -> &[f64] {
        &self.outputs
    }
    /// Output efficiency.
    pub fn efficiency(&self) -> f64 {
        self.efficiency
    }
    /// Computes a production plan without modifying any stocks.
    pub fn plan(
        &self,
        inputs: &[&Stock],
        outputs: &[&Stock],
        requested: f64,
    ) -> Result<RecipeReport> {
        nonnegative(requested)?;
        if inputs.len() != self.inputs.len() || outputs.len() != self.outputs.len() {
            return Err(Error::InvalidRecipe);
        }
        report(
            inputs
                .iter()
                .zip(&self.inputs)
                .map(|(stock, cost)| (stock.available(), *cost)),
            outputs
                .iter()
                .zip(&self.outputs)
                .map(|(stock, yield_)| (stock.receivable(), *yield_)),
            self.efficiency,
            requested,
        )
    }
    /// Plans and executes as one operation. All checks happen before mutation.
    pub fn execute(
        &self,
        inputs: &mut [&mut Stock],
        outputs: &mut [&mut Stock],
        requested: f64,
    ) -> Result<RecipeReport> {
        nonnegative(requested)?;
        if inputs.len() != self.inputs.len() || outputs.len() != self.outputs.len() {
            return Err(Error::InvalidRecipe);
        }
        let plan = report(
            inputs
                .iter()
                .zip(&self.inputs)
                .map(|(stock, cost)| (stock.available(), *cost)),
            outputs
                .iter()
                .zip(&self.outputs)
                .map(|(stock, yield_)| (stock.receivable(), *yield_)),
            self.efficiency,
            requested,
        )?;
        for (stock, cost) in inputs.iter_mut().zip(&self.inputs) {
            stock.withdraw(plan.completed * cost);
        }
        for (stock, yield_) in outputs.iter_mut().zip(&self.outputs) {
            stock.deposit(plan.completed * (yield_ * self.efficiency));
        }
        Ok(plan)
    }
}
fn validate(
    inputs: impl Iterator<Item = f64>,
    outputs: impl Iterator<Item = f64>,
    efficiency: f64,
) -> Result<()> {
    if !efficiency.is_finite() || !(0.0..=1.0).contains(&efficiency) {
        return Err(Error::InvalidRecipe);
    }
    for side in [inputs.collect::<Vec<_>>(), outputs.collect::<Vec<_>>()] {
        if !side.iter().any(|x| *x > 0.0) || side.iter().any(|x| !x.is_finite() || *x < 0.0) {
            return Err(Error::InvalidRecipe);
        }
    }
    Ok(())
}
fn report(
    inputs: impl Iterator<Item = (f64, f64)> + Clone,
    outputs: impl Iterator<Item = (f64, f64)> + Clone,
    efficiency: f64,
    requested: f64,
) -> Result<RecipeReport> {
    let mut runs = requested;
    for (available, cost) in inputs.clone() {
        if cost > 0.0 {
            runs = runs.min(available / cost);
        }
    }
    for (room, yield_) in outputs.clone() {
        let y = yield_ * efficiency;
        if y > 0.0 {
            runs = runs.min(room / y);
        }
    }
    let consumed = inputs.map(|(_, cost)| runs * cost).sum::<f64>();
    let produced = outputs
        .map(|(_, yield_)| runs * (yield_ * efficiency))
        .sum::<f64>();
    if !consumed.is_finite() || !produced.is_finite() {
        return Err(Error::Overflow);
    }
    Ok(RecipeReport {
        requested,
        completed: runs,
        consumed,
        produced,
    })
}
/// Allocation-free conversion using coefficients supplied inline.
/// Invalid coefficients, duration-like quantities, or efficiency cause no mutation.
pub fn convert_lossy(
    inputs: &mut [(&mut Stock, f64)],
    outputs: &mut [(&mut Stock, f64)],
    efficiency: f64,
    requested_runs: f64,
) -> f64 {
    if !requested_runs.is_finite()
        || requested_runs <= 0.0
        || !efficiency.is_finite()
        || !(0.0..=1.0).contains(&efficiency)
        || !inputs.iter().any(|(_, x)| *x > 0.0)
        || !outputs.iter().any(|(_, x)| *x > 0.0)
        || inputs
            .iter()
            .chain(outputs.iter())
            .any(|(_, x)| !x.is_finite() || *x < 0.0)
    {
        return 0.0;
    }
    let Ok(plan) = report(
        inputs
            .iter()
            .map(|(stock, cost)| (stock.available(), *cost)),
        outputs
            .iter()
            .map(|(stock, yield_)| (stock.receivable(), *yield_)),
        efficiency,
        requested_runs,
    ) else {
        return 0.0;
    };
    for (stock, cost) in inputs {
        stock.withdraw(plan.completed * *cost);
    }
    for (stock, yield_) in outputs {
        stock.deposit(plan.completed * (*yield_ * efficiency));
    }
    plan.completed
}
/// Inline conversion at full output efficiency.
pub fn convert(
    inputs: &mut [(&mut Stock, f64)],
    outputs: &mut [(&mut Stock, f64)],
    requested_runs: f64,
) -> f64 {
    convert_lossy(inputs, outputs, 1.0, requested_runs)
}
