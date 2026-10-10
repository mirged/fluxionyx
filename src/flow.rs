//! Resource exchange and losses; all amounts are constrained by reservoir bounds.
use crate::{
    Error, Result, Stock,
    error::{nonnegative, positive},
};

/// Transfers up to a finite amount; conservation holds within floating-point rounding.
pub fn flow(source: &mut Stock, target: &mut Stock, amount: f64) -> f64 {
    if !amount.is_finite() || amount <= 0.0 {
        return 0.0;
    }
    let moved = source.withdraw(amount.min(target.receivable()));
    target.deposit(moved);
    moved
}
/// Transfers a clamped fraction of the source's withdrawable quantity.
pub fn flow_ratio(source: &mut Stock, target: &mut Stock, fraction: f64) -> f64 {
    if !fraction.is_finite() {
        return 0.0;
    }
    flow(
        source,
        target,
        source.available() * fraction.clamp(0.0, 1.0),
    )
}
/// Applies a transfer rate over an explicit duration.
pub fn flow_rate(source: &mut Stock, target: &mut Stock, rate: f64, dt: f64) -> f64 {
    if !rate.is_finite() || !dt.is_finite() || rate <= 0.0 || dt <= 0.0 {
        return 0.0;
    }
    flow(source, target, (rate * dt).min(f64::MAX))
}
/// Linear loss. Returns the actual withdrawn amount.
pub fn leak(stock: &mut Stock, rate: f64, dt: f64) -> f64 {
    if !rate.is_finite() || !dt.is_finite() || rate <= 0.0 || dt <= 0.0 {
        return 0.0;
    }
    stock.withdraw((rate * dt).min(f64::MAX))
}
/// Half-life decay toward the stock's lower bound.
pub fn evaporate(stock: &mut Stock, half_life: f64, dt: f64) -> f64 {
    if !half_life.is_finite() || !dt.is_finite() || half_life <= 0.0 || dt <= 0.0 {
        return 0.0;
    }
    stock.withdraw(stock.available() * -libm::expm1(-core::f64::consts::LN_2 * dt / half_life))
}
/// Exchanges resources at `price` units of payment per unit of goods.
/// Both transfers are limited together; invalid prices cause no mutation.
pub fn trade(
    goods: &mut Stock,
    buyer_goods: &mut Stock,
    payment: &mut Stock,
    seller_payment: &mut Stock,
    requested: f64,
    price: f64,
) -> Result<f64> {
    nonnegative(requested)?;
    positive(price)?;
    let amount = requested
        .min(goods.available())
        .min(buyer_goods.receivable())
        .min(payment.available() / price)
        .min(seller_payment.receivable() / price);
    if !(amount * price).is_finite() {
        return Err(Error::Overflow);
    }
    let received = flow(goods, buyer_goods, amount);
    flow(payment, seller_payment, received * price);
    Ok(received)
}
