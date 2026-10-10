//! Optional fixed-point resource accounting, enabled by the `fixed` feature.
//! This does not change the floating-point modules or guarantee their replay portability.
use crate::{Error, Result};
pub use ::fixed::types::I32F32 as Amount;

/// Nonnegative bounded fixed-point stock with exact integer-bit transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedStock {
    current: Amount,
    capacity: Amount,
}
impl FixedStock {
    /// Constructs a stock within a nonnegative representable capacity.
    pub fn new(initial: Amount, capacity: Amount) -> Result<Self> {
        if capacity < Amount::ZERO {
            return Err(Error::InvalidBounds);
        }
        if initial < Amount::ZERO || initial > capacity {
            return Err(Error::OutOfBounds);
        }
        Ok(Self {
            current: initial,
            capacity,
        })
    }
    /// Current quantity.
    pub fn val(&self) -> Amount {
        self.current
    }
    /// Maximum quantity.
    pub fn capacity(&self) -> Amount {
        self.capacity
    }
    /// Available capacity.
    pub fn room(&self) -> Amount {
        self.capacity - self.current
    }
    /// Adds up to capacity; returns actual accepted quantity.
    pub fn deposit(&mut self, amount: Amount) -> Amount {
        let accepted = amount.max(Amount::ZERO).min(self.room());
        self.current += accepted;
        accepted
    }
    /// Removes up to the current quantity; returns actual removal.
    pub fn withdraw(&mut self, amount: Amount) -> Amount {
        let removed = amount.max(Amount::ZERO).min(self.current);
        self.current -= removed;
        removed
    }
    /// Conserved transfer, exact to the last fixed-point bit.
    pub fn flow_to(&mut self, target: &mut Self, amount: Amount) -> Amount {
        let moved = self.withdraw(amount.min(target.room()));
        target.deposit(moved);
        moved
    }
}
