//! Scalar grids with conservative diffusion and bilinear transport.
use crate::{
    Error, Result,
    error::{finite, nonnegative},
};
use alloc::{vec, vec::Vec};

/// Edge behavior for sampling, diffusion, and advection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Boundary {
    /// Reflects diffusion flux and clamps sampling at edges.
    #[default]
    Closed,
    /// Wraps around opposite edges.
    Periodic,
}
/// Dense row-major field with a reusable scratch buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct Field2D {
    width: usize,
    height: usize,
    cells: Vec<f64>,
    scratch: Vec<f64>,
    boundary: Boundary,
}
impl Field2D {
    /// Creates a closed field; panics for invalid dimensions or value.
    pub fn new(width: usize, height: usize, value: f64) -> Self {
        Self::try_new(width, height, value).expect("invalid field configuration")
    }
    /// Checked constructor; empty or overflowing grids are rejected.
    pub fn try_new(width: usize, height: usize, value: f64) -> Result<Self> {
        finite(value)?;
        let size = width.checked_mul(height).ok_or(Error::InvalidDimensions)?;
        if size == 0 || size > isize::MAX as usize / core::mem::size_of::<f64>() {
            return Err(Error::InvalidDimensions);
        }
        Ok(Self {
            width,
            height,
            cells: vec![value; size],
            scratch: vec![value; size],
            boundary: Boundary::Closed,
        })
    }
    /// Selects boundary behavior.
    pub fn with_boundary(mut self, boundary: Boundary) -> Self {
        self.boundary = boundary;
        self
    }
    /// Grid width.
    pub fn width(&self) -> usize {
        self.width
    }
    /// Grid height.
    pub fn height(&self) -> usize {
        self.height
    }
    /// Boundary behavior.
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    /// Row-major cell values.
    pub fn cells(&self) -> &[f64] {
        &self.cells
    }
    /// Reads a cell; coordinates outside the grid return zero.
    pub fn get(&self, x: usize, y: usize) -> f64 {
        self.try_get(x, y).unwrap_or(0.0)
    }
    /// Checked cell read.
    pub fn try_get(&self, x: usize, y: usize) -> Option<f64> {
        if x < self.width && y < self.height {
            Some(self.cells[y * self.width + x])
        } else {
            None
        }
    }
    /// Writes a finite cell value; invalid coordinates/values are ignored.
    pub fn set(&mut self, x: usize, y: usize, value: f64) {
        let _ = self.try_set(x, y, value);
    }
    /// Checked cell write.
    pub fn try_set(&mut self, x: usize, y: usize, value: f64) -> Result<()> {
        finite(value)?;
        if x >= self.width || y >= self.height {
            return Err(Error::OutOfBounds);
        }
        self.cells[y * self.width + x] = value;
        Ok(())
    }
    /// Replaces every cell with a finite value.
    pub fn fill(&mut self, value: f64) -> Result<()> {
        finite(value)?;
        self.cells.fill(value);
        Ok(())
    }
    /// Sum of all cells; approximately conserved by diffusion.
    pub fn total(&self) -> f64 {
        self.cells.iter().sum()
    }
    /// Mean cell value.
    pub fn mean(&self) -> f64 {
        self.cells.iter().map(|x| x / self.cells.len() as f64).sum()
    }
    /// Minimum and maximum cell values.
    pub fn range(&self) -> (f64, f64) {
        self.cells
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), x| {
                (min.min(*x), max.max(*x))
            })
    }
    /// Bilinear sampling with the field's boundary mode. Invalid coordinates return zero.
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        if !x.is_finite() || !y.is_finite() {
            return 0.0;
        }
        let wrap = |v: f64, n: usize| {
            let n = n as f64;
            let r = v % n;
            if r < 0.0 { r + n } else { r }
        };
        let (x, y) = match self.boundary {
            Boundary::Closed => (
                x.clamp(0.0, (self.width - 1) as f64),
                y.clamp(0.0, (self.height - 1) as f64),
            ),
            Boundary::Periodic => (wrap(x, self.width), wrap(y, self.height)),
        };
        let x0 = libm::floor(x) as usize;
        let y0 = libm::floor(y) as usize;
        let (x1, y1) = match self.boundary {
            Boundary::Closed => ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1)),
            Boundary::Periodic => ((x0 + 1) % self.width, (y0 + 1) % self.height),
        };
        let tx = x - x0 as f64;
        let ty = y - y0 as f64;
        crate::curves::lerp(
            crate::curves::lerp(self.get(x0, y0), self.get(x1, y0), tx),
            crate::curves::lerp(self.get(x0, y1), self.get(x1, y1), tx),
            ty,
        )
    }
    /// Explicit heat diffusion with stable substeps on unit-spaced cells.
    /// Supports single-row/column grids. Rejects more than 1,000,000 substeps
    /// before mutation; caller can reduce duration/rate. No allocations per pass.
    pub fn try_diffuse(&mut self, rate: f64, dt: f64) -> Result<()> {
        nonnegative(rate)?;
        nonnegative(dt)?;
        if rate == 0.0 || dt == 0.0 {
            return Ok(());
        }
        let amount = rate * dt;
        let steps_f = libm::ceil(4.0 * amount).max(1.0);
        if !steps_f.is_finite() || steps_f > 1_000_000.0 {
            return Err(Error::Overflow);
        }
        let steps = steps_f as usize;
        let alpha = amount / steps as f64;
        for _ in 0..steps {
            for y in 0..self.height {
                for x in 0..self.width {
                    let (left, right, up, down) = match self.boundary {
                        Boundary::Closed => (
                            x.saturating_sub(1),
                            (x + 1).min(self.width - 1),
                            y.saturating_sub(1),
                            (y + 1).min(self.height - 1),
                        ),
                        Boundary::Periodic => (
                            if x == 0 { self.width - 1 } else { x - 1 },
                            (x + 1) % self.width,
                            if y == 0 { self.height - 1 } else { y - 1 },
                            (y + 1) % self.height,
                        ),
                    };
                    // A convex combination preserves positivity and avoids large neighbor sums.
                    self.scratch[y * self.width + x] = (1.0 - 4.0 * alpha) * self.get(x, y)
                        + alpha * self.get(left, y)
                        + alpha * self.get(right, y)
                        + alpha * self.get(x, up)
                        + alpha * self.get(x, down);
                }
            }
            core::mem::swap(&mut self.cells, &mut self.scratch);
        }
        Ok(())
    }
    /// Convenience diffusion; rejected parameters leave the grid unchanged.
    pub fn diffuse(&mut self, rate: f64, dt: f64) {
        let _ = self.try_diffuse(rate, dt);
    }
    /// Semi-Lagrangian transport at a uniform velocity; bilinear interpolation
    /// preserves constant fields but does not promise mass conservation.
    pub fn advect(&mut self, velocity_x: f64, velocity_y: f64, dt: f64) {
        if ![velocity_x, velocity_y, dt, velocity_x * dt, velocity_y * dt]
            .iter()
            .all(|x| x.is_finite())
            || dt <= 0.0
        {
            return;
        }
        for y in 0..self.height {
            for x in 0..self.width {
                self.scratch[y * self.width + x] =
                    self.sample(x as f64 - velocity_x * dt, y as f64 - velocity_y * dt);
            }
        }
        core::mem::swap(&mut self.cells, &mut self.scratch);
    }
    /// Multiplies every cell by a finite factor clamped to `[0, 1]`.
    pub fn decay_all(&mut self, factor: f64) {
        if factor.is_finite() {
            for cell in &mut self.cells {
                *cell *= factor.clamp(0.0, 1.0);
            }
        }
    }
    /// Half-life decay over a finite positive duration.
    pub fn evaporate(&mut self, half_life: f64, dt: f64) {
        if half_life.is_finite() && dt.is_finite() && half_life > 0.0 && dt > 0.0 {
            self.decay_all(libm::exp(-core::f64::consts::LN_2 * dt / half_life));
        }
    }
}
