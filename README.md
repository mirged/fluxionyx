# Fluxionyx

Composable simulation building blocks for economies, populations, environments,
and game systems. Fluxionyx turns the original `sim_core` demo into a Rust library
with explicit time, bounded resources, and useful defaults.

## Installation

```toml
[dependencies]
fluxionyx = "0.1.0"
```

Enable the optional `fixed` feature for fixed-point stocks and transfers. Set
`default-features = false` for `no_std` with `alloc`.

[API documentation](https://docs.rs/fluxionyx) ·
[crates.io](https://crates.io/crates/fluxionyx) ·
[GitHub](https://github.com/mirged/fluxionyx)

```rust
use fluxionyx::prelude::*;

let mut ore = Stock::new(100.0)?;
let mut coal = Stock::new(40.0)?;
let mut steel = Stock::bounded(0.0, 25.0, 0.0)?;
let smelter = Recipe::new(&[2.0, 1.0], &[1.0], 0.9)?;
let mut clock = Clock::new(0.25)?;

for _ in 0..4 {
    let step = clock.advance()?;
    let report = smelter.execute(
        &mut [&mut ore, &mut coal],
        &mut [&mut steel],
        8.0 * step.dt,
    )?;
    assert_eq!(report.completed, 2.0);
}
assert!((steel.val() - 7.2).abs() < 1e-10);
# Ok::<(), fluxionyx::Error>(())
```

## Doctrine: explicit time, bounded state, composable operations

1. **Keep ownership in your model.** Stocks, fields, cohorts, queues, and stats
   are ordinary Rust values. No global world, hidden scheduler, or mandatory ECS.
2. **Choose units once.** A duration may represent seconds, days, or years; all
   rates in a model must use the same time unit. `Clock` never reads wall time.
3. **Separate rates from quantities.** `inflow`/`outflow` accept quantities per
   time; `integrate(dt)` applies them once and clears pending rates. `logistic`
   returns a delta already multiplied by time: deposit that delta directly.
4. **Protect boundaries.** Stateful containers keep their internals private.
   Constructors and checked mutations return a shared `Error`. Rejected checked
   operations leave state unchanged unless their documentation says otherwise.
5. **Make limits observable.** Transfers return actual quantities. Integration
   reports clipping. Recipes support previews and report completed batches.
6. **Compose in a deliberate order.** Read state, decide rates and operations,
   execute transfers/recipes, integrate rates, then observe. Operations are
   sequential; the caller defines contention priority. A recipe validates and
   plans every binding before it mutates any stock.
7. **Pay for what you use.** Stocks, flow operations, curves, clocks, RNG, and
   online statistics do not allocate. Cohorts and fields allocate during setup
   and reuse buffers; stats and queues can allocate when adding new entries.

## Modules

| Module | Use |
| --- | --- |
| `stock`, `flow` | Reservoirs, rates, transfers, barter, leaks, half-life decay |
| `recipe` | Reusable production coefficients, dry-run plans, constrained execution |
| `curves` | Logistic growth, saturation, sigmoid, smoothstep, interpolation |
| `stat` | Named flat, additive percent, and multiplicative modifiers; cached values |
| `population` | Seeded age brackets, migration, mortality, fractional ageing |
| `spatial` | Closed/periodic grids, sampling, stable diffusion, advection |
| `time` | Tick-count clocks, frame-time accumulation, interpolation fraction |
| `schedule` | Typed events, stable tie ordering, cancellation, dispatch |
| `random` | Seeded draws, unbiased indices, shuffles, weighted selection |
| `statistics` | Online/mergeable mean and variance, bounded rolling history |
| `control` | Half-life smoothing, hysteresis, tick delay lines |
| `fixed` | Optional fixed-point resource stocks and exact transfers |

Use `fluxionyx::prelude::*` to get common types and functions, or import modules
individually. Generate complete API documentation with `cargo doc --open`.
See [the API guide](API_REFERENCE.md) for a compact tour of the call conventions.

## Features and numerical behavior

- Default `std` feature supports ordinary applications.
- `default-features = false` provides `no_std` **with `alloc`**. A global allocator
  is needed for containers; scalar primitives themselves do not allocate.
  `libm` supplies floating-point transcendental functions in both modes.
- `fixed` adds `fixed::FixedStock` and `fixed::Amount` (signed 32.32 values).
  It does not replace the `f64` API. Fixed-point transfers conserve bits exactly;
  floating-point calculations do not promise cross-platform bit-identical replay.
- Seeded RNG integer draws and event tie ordering are repeatable. Record the
  seed, fixed tick duration, inputs, and operation order for replay.
- Curves are pure numeric helpers: callers are responsible for finite arguments
  and appropriate domains unless a function documents domain handling. Stats
  may overflow during aggregation of extreme finite modifiers.
- Floating-point transfers and diffusion conserve within rounding error.
  Diffusion uses unit cell spacing and substeps enforcing `rate * sub_dt <= 0.25`.
  More than one million substeps is rejected before mutation.
  Semi-Lagrangian advection preserves uniform fields but is not mass-conservative.
- Cohort counts assume uniform distribution inside brackets. Fractional ageing
  redistributes that approximation each call; splitting a step can add numerical
  spreading. Counts beyond the age horizon are returned as departures.
- Convenience methods such as `tick`, `diffuse`, `births`, and inline `convert`
  ignore invalid requests. Use checked variants for externally supplied input.

## Run and verify

```text
cargo run --example imperial
cargo run --example village
cargo test --all-features
cargo test --no-default-features
cargo check --no-default-features --features fixed
cargo clippy --all-features --all-targets -- -D warnings
```

The imperial scenario is retained from the original demo. The village example
shows multiple modules working together. Neither scenario is historical data.

## Migration from sim_core

The package and directory are now `fluxionyx`. Add a path dependency pointing to
this directory and replace `mod core; use core::*;` with
`use fluxionyx::prelude::*;`. The old executable is now `examples/imperial.rs`.
Most primitive call names remain available. Constructors now return typed errors;
stock/field internals are private and expose getters. `working_slice` uses
half-open age ranges with weighted partial brackets, and `advance` handles long
durations. Half-life decay approaches a stock's lower bound. The former unused
`fixed` feature now exposes a separate, working fixed-point stock.
