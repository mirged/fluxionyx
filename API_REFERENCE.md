# Fluxionyx API guide

Import common operations with `use fluxionyx::prelude::*;`. Checked APIs return
`fluxionyx::Result<T>`. Generate full signatures and contracts with `cargo doc`.

## Resources and accounting

`Stock::new(initial)` creates a nonnegative, unbounded stock.
`Stock::bounded(min, max, initial)` creates a bounded stock. Both reject
NaN, infinity, and invalid starting state. Use `val`, `min`, `max`, `available`,
`room`, `ratio`, `is_empty`, and `is_full` for inspection; state is private.
Unbounded stocks still cannot exceed the largest representable `f64`.

`deposit` and `withdraw` return actual quantities. `set` checks a replacement
against bounds. Register quantities **per time unit** with `inflow` and
`outflow`, then call `integrate(dt)` exactly once per step. Its `TickReport`
contains `requested`, `applied`, and signed `clipped` deltas. Successful steps
clear rates; rejected steps retain them. `tick` ignores integration errors.

`flow`, `flow_ratio`, and `flow_rate` constrain transfers by source reserves
and destination capacity. `flow_ratio` uses the source's **available quantity**,
excluding its lower bound. `trade` constrains goods and payment together.
`leak` removes a linear rate; `evaporate` decays quantity above the lower bound
using a half-life. These operations return actual removal or transfer.

## Recipes

`Recipe::new(input_coefficients, output_coefficients, efficiency)` owns reusable
coefficients. Every coefficient must be finite and nonnegative, each side must
have a positive coefficient, and efficiency is in `[0, 1]`. Zero efficiency
explicitly permits consuming all inputs as waste.

`plan(&[&Stock], &[&Stock], requested)` previews production. `execute` binds
`&mut [&mut Stock]` in the same coefficient order, validates all bindings before
mutation, and returns the same plan as a report. Stocks cannot be aliased in one
safe Rust call. Requests may be fractional batches. Reports contain requested
and completed batches and aggregate consumed/produced quantities; aggregating
different resource units is only meaningful when the application defines it.

`convert` and `convert_lossy` accept inline `(&mut Stock, coefficient)` tuples
without allocations. Invalid inline requests return zero without mutation.

## Response and modifiers

`logistic` returns an Euler **delta** including `dt`; `logistic_next` returns an
exact next population for its documented nonnegative domain. `diminishing`,
`s_curve`, `soft_cap`, `elastic`, `lerp`, `smoothstep`, and `approach` provide
response functions. Supply finite arguments and coherent units.

`Stat` evaluates `(base + sum(flat)) * (1 + sum(percent)) * product(factors)`.
Use `add`, `percent`, and `mult` with named tags. Updating a tag within one layer
replaces its value while retaining insertion order. `remove` removes all entries
with that tag. `set_base`, `set_bounds`, `clear_bounds`, and `clear` update cached
aggregation. Tags may be dynamically allocated strings. Extreme finite entries
can overflow the aggregate; output bounds clamp after aggregation.

## Populations and fields

`Cohort::from_buckets(counts, bracket_years)` seeds age brackets directly.
`try_new` is checked; `new` panics for invalid configuration. `births` adds to
the youngest bracket. `migrate` adds/removes population at a bracket and returns
the signed change. `cull_rate` uses linear mortality; `cull_hazard` uses continuous
survival. `advance` handles arbitrary positive durations and returns people who
left the age horizon. `working_slice` weights overlap with `[min_age, max_age)`.
`buckets`, `total`, `bucket_years`, `max_age`, and `mean_age` provide observations.
Fractional operations assume uniform population within each bracket.

`Field2D` is a row-major field with private dimensions and buffers. `try_new`,
`try_get`, and `try_set` expose checked operations; `new` panics for invalid
configuration, `get` returns zero outside the grid, and `set` ignores errors.
Use `with_boundary(Boundary::Periodic)` for wrapping, or the default `Closed`.
`sample` performs bilinear interpolation using that boundary mode. `try_diffuse`
uses stable explicit substeps with unit spacing, including 1D grids, and rejects
more than a million substeps. `diffuse` ignores errors. `advect` uses a uniform
velocity in cells per time; it is not mass-conservative. `fill`, `cells`, `range`,
`mean`, `total`, `decay_all`, and half-life `evaporate` support field operations.

## Time and events

`Clock::new(dt)` starts at tick zero. `advance` returns a `Step` with one-based
`index`, fixed `dt`, and elapsed time **after** the step. `FixedStepper` accepts
frame time via `accumulate`; repeatedly call `next_step` to consume whole steps.
`alpha` is the interpolation fraction once the backlog is drained. The caller
sets a per-frame processing budget; backlog is retained. Decimal durations have
an eight-epsilon roundoff tolerance when checking whether a step is available.

`EventQueue<E>` owns arbitrary typed payloads. `schedule_at` uses absolute time;
`schedule_after` uses current queue time. Same-time events run in insertion
order. `cancel` removes pending events by queue-local `EventId`. `advance_to`
returns all due events through its inclusive endpoint and advances to that time.
If handlers need to schedule relative to individual event times, use `peek` and
`pop_next` to dispatch events one at a time. Events earlier than queue time are
rejected. The queue never reads wall time.

## Randomness, observations, and feedback

`Rng::new(seed)` uses SplitMix64. `next_u64` and `state` support integer replay
and snapshots. `unit`, `range`, `index`, `chance`, `shuffle`, `weighted_index`,
`exponential`, and `normal` cover common random decisions. It is not a
cryptographic generator. Floating-point distributions do not promise
cross-platform bit-identical results.

`RunningStats` uses constant-memory online mean/variance. `push` validates an
observation; `merge` combines accumulators without retaining samples. `mean`,
`variance`, `sample_variance`, `std_dev`, and `range` return `None` without enough
observations. `Window::new(capacity)` retains bounded history; `push` returns any
evicted value, `values` iterates oldest to newest, and `mean` avoids rolling-sum
drift.

`Smoother` approaches targets according to half-life, `Threshold` retains its
boolean state between separate activation/deactivation thresholds, and
`Delay<T>` returns observations from an explicit number of pushes earlier.

## Optional fixed-point accounting

Enable `fixed` to use `fluxionyx::fixed::{Amount, FixedStock}`. `Amount` is signed
32.32 fixed point. Stocks themselves are nonnegative and bounded; use `val`,
`capacity`, `room`, `deposit`, `withdraw`, and `flow_to`. Transfers conserve raw
integer bits exactly. This feature does not change the floating-point API.
