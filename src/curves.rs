//! Pure response curves. Callers supply consistent units; rates and deltas are distinct.

/// Euler logistic growth **delta**, already multiplied by `dt`.
pub fn logistic(current: f64, capacity: f64, rate: f64, dt: f64) -> f64 {
    if capacity <= 0.0 || current <= 0.0 || dt <= 0.0 {
        return 0.0;
    }
    rate * current * (1.0 - current / capacity) * dt
}
/// Exact logistic next value for nonnegative rate, population, and elapsed time.
/// Invalid domains return the original population.
pub fn logistic_next(current: f64, capacity: f64, rate: f64, dt: f64) -> f64 {
    if ![current, capacity, rate, dt].iter().all(|x| x.is_finite())
        || current <= 0.0
        || capacity <= 0.0
        || rate < 0.0
        || dt <= 0.0
    {
        return current;
    }
    // Reciprocal form avoids overflow for very small positive populations.
    let e = libm::exp(-rate * dt);
    current / (e + (current / capacity) * (1.0 - e))
}
/// Michaelis-Menten saturation in `[0, 1]`.
pub fn diminishing(input: f64, half_max: f64) -> f64 {
    if input <= 0.0 || half_max <= 0.0 {
        return 0.0;
    }
    1.0 / (1.0 + half_max / input)
}
/// Numerically stable logistic sigmoid.
pub fn s_curve(x: f64, midpoint: f64, steepness: f64) -> f64 {
    let z = steepness * (x - midpoint);
    if z >= 0.0 {
        1.0 / (1.0 + libm::exp(-z))
    } else {
        let e = libm::exp(z);
        e / (1.0 + e)
    }
}
/// Smooth ceiling approaching `limit + softness` above the limit.
pub fn soft_cap(value: f64, limit: f64, softness: f64) -> f64 {
    if softness <= 0.0 || value <= limit {
        value
    } else {
        limit + softness * libm::tanh((value - limit) / softness)
    }
}
/// Supply/demand response; zero supply is floored at `1e-9`.
pub fn elastic(demand: f64, supply: f64, base_price: f64, sensitivity: f64) -> f64 {
    if base_price <= 0.0 {
        return 0.0;
    }
    base_price * libm::pow(demand.max(0.0) / supply.max(1e-9), sensitivity)
}
/// Unclamped linear interpolation.
pub fn lerp(start: f64, end: f64, fraction: f64) -> f64 {
    (1.0 - fraction) * start + fraction * end
}
/// Clamped cubic interpolation with zero slope at both ends.
///
/// Returns 0 at `start` and 1 at `end`. Reversed edges (`start > end`) give a falling curve,
/// matching common shader `smoothstep` usage; a zero-width interval is a step at `start`.
pub fn smoothstep(start: f64, end: f64, value: f64) -> f64 {
    if start == end {
        return if value < start { 0.0 } else { 1.0 };
    }
    let t = ((value - start) / (end - start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
/// Time-based exponential approach, parameterized by positive half-life.
pub fn approach(current: f64, target: f64, half_life: f64, dt: f64) -> f64 {
    if !half_life.is_finite() || !dt.is_finite() || half_life <= 0.0 || dt <= 0.0 {
        return current;
    }
    lerp(
        current,
        target,
        -libm::expm1(-core::f64::consts::LN_2 * dt / half_life),
    )
}
