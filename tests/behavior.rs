use fluxionyx::prelude::*;

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0),
        "{actual} != {expected}"
    );
}

#[test]
fn constructors_reject_nonfinite_or_inconsistent_state() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Stock::new(bad).is_err());
        assert!(Stock::bounded(0.0, 1.0, bad).is_err());
        assert!(Clock::new(bad).is_err());
        assert!(Cohort::try_new(5, bad).is_err());
        assert!(Field2D::try_new(2, 2, bad).is_err());
    }
    assert!(Stock::bounded(2.0, 1.0, 1.0).is_err());
    assert!(Stock::bounded(0.0, 1.0, 2.0).is_err());
    assert!(Stock::bounded(-f64::MAX, f64::MAX, 0.0).is_err());
    assert!(Cohort::try_new(0, 1.0).is_err());
    assert!(Field2D::try_new(usize::MAX, 2, 0.0).is_err());
}

#[test]
fn resource_transfer_preserves_total_and_reserves() -> Result<()> {
    let mut a = Stock::bounded(10.0, 100.0, 50.0)?;
    let mut b = Stock::bounded(5.0, 40.0, 5.0)?;
    close(flow(&mut a, &mut b, 100.0), 35.0);
    close(a.val() + b.val(), 55.0);
    assert!(b.is_full());
    b.withdraw(30.0);
    close(flow_ratio(&mut a, &mut b, 1.0), 5.0);
    assert!(a.is_empty());
    close(a.val(), 10.0);
    Ok(())
}

#[test]
fn random_transfers_preserve_bounds_and_total() -> Result<()> {
    let mut a = Stock::bounded(0.0, 400.0, 200.0)?;
    let mut b = Stock::bounded(0.0, 400.0, 200.0)?;
    let mut rng = Rng::new(17);
    for i in 0..10_000 {
        let amount = rng.range(0.0, 500.0)?;
        if i % 2 == 0 {
            flow(&mut a, &mut b, amount);
        } else {
            flow(&mut b, &mut a, amount);
        }
        assert!((0.0..=400.0).contains(&a.val()));
        assert!((0.0..=400.0).contains(&b.val()));
        close(a.val() + b.val(), 400.0);
    }
    Ok(())
}

#[test]
fn checked_integration_reports_clipping_and_retains_failed_rates() -> Result<()> {
    let mut stock = Stock::bounded(0.0, 10.0, 8.0)?;
    stock.inflow(5.0);
    stock.outflow(1.0);
    let before = stock.clone();
    assert!(stock.integrate(f64::NAN).is_err());
    assert_eq!(stock, before);
    let report = stock.integrate(1.0)?;
    close(report.requested, 4.0);
    close(report.applied, 2.0);
    close(report.clipped, 2.0);
    assert_eq!(stock.rates(), (0.0, 0.0));
    stock.inflow(f64::INFINITY);
    stock.outflow(f64::NAN);
    assert_eq!(stock.rates(), (0.0, 0.0));
    assert!(stock.set(-1.0).is_err());
    close(stock.val(), 10.0);
    Ok(())
}

#[test]
fn half_life_decay_is_composable_and_respects_signed_minimum() -> Result<()> {
    let mut one = Stock::bounded(-20.0, 100.0, 80.0)?;
    let mut two = one.clone();
    close(one.evaporate(2.0, 2.0), 50.0);
    two.evaporate(2.0, 1.0);
    two.evaporate(2.0, 1.0);
    close(one.val(), two.val());
    close(one.val(), 30.0);
    close(one.leak(1000.0, 1.0), 50.0);
    close(one.val(), -20.0);
    Ok(())
}

#[test]
fn trade_is_limited_by_buyer_funds_and_seller_capacity() -> Result<()> {
    let mut goods = Stock::new(100.0)?;
    let mut received = Stock::new(0.0)?;
    let mut funds = Stock::new(30.0)?;
    let mut revenue = Stock::bounded(0.0, 20.0, 0.0)?;
    close(
        trade(
            &mut goods,
            &mut received,
            &mut funds,
            &mut revenue,
            100.0,
            2.0,
        )?,
        10.0,
    );
    close(goods.val() + received.val(), 100.0);
    close(funds.val() + revenue.val(), 30.0);
    close(received.val() * 2.0, revenue.val());
    assert!(
        trade(
            &mut goods,
            &mut received,
            &mut funds,
            &mut revenue,
            2.0,
            -1.0
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn recipe_preview_and_execution_match_output_limit() -> Result<()> {
    let mut ore = Stock::bounded(10.0, 200.0, 100.0)?;
    let mut coal = Stock::new(50.0)?;
    let mut steel = Stock::bounded(0.0, 5.0, 0.0)?;
    let recipe = Recipe::new(&[2.0, 1.0], &[1.0], 0.5)?;
    let preview = recipe.plan(&[&ore, &coal], &[&steel], 30.0)?;
    close(preview.completed, 10.0);
    close(ore.val(), 100.0);
    let result = recipe.execute(&mut [&mut ore, &mut coal], &mut [&mut steel], 30.0)?;
    assert_eq!(preview, result);
    close(ore.val(), 80.0);
    close(coal.val(), 40.0);
    close(steel.val(), 5.0);
    Ok(())
}

#[test]
fn invalid_inline_recipes_never_mutate_any_binding() -> Result<()> {
    for coefficient in [-1.0, f64::NAN, f64::INFINITY] {
        let mut ore = Stock::new(10.0)?;
        let mut steel = Stock::new(0.0)?;
        close(
            convert_lossy(
                &mut [(&mut ore, coefficient)],
                &mut [(&mut steel, 1.0)],
                0.8,
                4.0,
            ),
            0.0,
        );
        close(ore.val(), 10.0);
        close(steel.val(), 0.0);
        assert!(Recipe::new(&[coefficient], &[1.0], 0.8).is_err());
    }
    assert!(Recipe::new(&[0.0], &[1.0], 1.0).is_err());
    let mut ore = Stock::new(10.0)?;
    let mut output = Stock::new(0.0)?;
    let recipe = Recipe::new(&[1.0], &[1.0], 1.0)?;
    assert!(recipe.execute(&mut [], &mut [&mut output], 2.0).is_err());
    close(output.val(), 0.0);
    close(
        convert_lossy(&mut [(&mut ore, 1.0)], &mut [(&mut output, 1.0)], 0.0, 2.0),
        2.0,
    );
    close(ore.val(), 8.0);
    close(output.val(), 0.0); // Full waste is explicitly supported.
    Ok(())
}

#[test]
fn stat_layers_replace_tags_and_preserve_bounds() -> Result<()> {
    let mut stat = Stat::new(100.0);
    stat.add(String::from("tools"), 10.0);
    stat.add("tools", 20.0);
    stat.percent("policy", 0.1);
    stat.percent("tech", 0.2);
    stat.mult("morale", 0.5);
    close(stat.val(), 78.0);
    assert_eq!(stat.modifier_count(), 4);
    stat.set_base(200.0)?;
    close(stat.val(), 143.0);
    stat.set_bounds(0.0, 100.0)?;
    close(stat.val(), 100.0);
    assert!(stat.remove("policy"));
    assert!(!stat.remove("absent"));
    stat.clear();
    close(stat.val(), 100.0);
    stat.clear_bounds();
    close(stat.val(), 200.0);
    Ok(())
}

#[test]
fn cohort_ages_multiple_brackets_and_reports_departures() -> Result<()> {
    let mut cohort = Cohort::from_buckets(&[100.0, 40.0, 20.0, 10.0], 5.0)?;
    let lost = cohort.advance(12.5);
    assert_eq!(cohort.buckets(), &[0.0, 0.0, 50.0, 70.0]);
    close(lost, 50.0);
    close(cohort.total() + lost, 170.0);
    close(cohort.advance(100.0), 120.0);
    close(cohort.total(), 0.0);
    Ok(())
}

#[test]
fn age_slices_weight_overlap_and_migration_is_checked() -> Result<()> {
    let mut cohort = Cohort::from_buckets(&[100.0, 100.0, 100.0], 5.0)?;
    close(cohort.working_slice(2.5, 7.5), 100.0);
    close(cohort.working_slice(5.0, 10.0), 100.0);
    close(cohort.working_slice(-5.0, 5.0), 100.0);
    close(cohort.mean_age().unwrap(), 7.5);
    close(cohort.migrate(1, -200.0)?, -100.0);
    assert!(cohort.migrate(10, 1.0).is_err());
    cohort.births(f64::INFINITY);
    close(cohort.total(), 200.0);
    Ok(())
}

#[test]
fn continuous_mortality_obeys_half_life() -> Result<()> {
    let mut cohort = Cohort::from_buckets(&[100.0, 200.0], 1.0)?;
    close(cohort.cull_hazard(|_| core::f64::consts::LN_2, 1.0), 150.0);
    close(cohort.total(), 150.0);
    close(cohort.cull_rate(|_| 5.0, 1.0), 150.0);
    close(cohort.total(), 0.0);
    Ok(())
}

#[test]
fn diffusion_conserves_mass_and_nonnegativity_in_each_boundary_mode() -> Result<()> {
    for boundary in [Boundary::Closed, Boundary::Periodic] {
        for (w, h) in [(1, 1), (8, 1), (1, 8), (5, 7)] {
            let mut field = Field2D::new(w, h, 0.0).with_boundary(boundary);
            field.set(0, 0, 100.0);
            field.try_diffuse(0.7, 3.0)?;
            close(field.total(), 100.0);
            assert!(field.cells().iter().all(|x| *x >= 0.0));
            if w * h > 1 {
                assert!(field.get(0, 0) < 100.0);
            }
            let before = field.clone();
            assert!(field.try_diffuse(f64::MAX, 2.0).is_err());
            assert_eq!(field, before);
        }
    }
    Ok(())
}

#[test]
fn field_sampling_wraps_and_advection_preserves_constants() {
    let mut field = Field2D::new(2, 1, 0.0).with_boundary(Boundary::Periodic);
    field.set(1, 0, 10.0);
    close(field.sample(0.5, 0.0), 5.0);
    close(field.sample(-0.5, 0.0), 5.0);
    field.advect(1.0, 0.0, 1.0);
    assert_eq!(field.cells(), &[10.0, 0.0]);
    let mut constant = Field2D::new(1, 5, 12.0);
    constant.advect(2.0, 1.0, 4.0);
    assert_eq!(constant.cells(), &[12.0; 5]);
    assert_eq!(constant.try_get(9, 0), None);
}

#[test]
fn clock_and_stepper_commit_only_valid_time() -> Result<()> {
    let mut stepper = FixedStepper::new(0.25)?;
    stepper.accumulate(0.625)?;
    assert_eq!(stepper.next_step()?.unwrap().index, 1);
    close(stepper.next_step()?.unwrap().elapsed, 0.5);
    assert!(stepper.next_step()?.is_none());
    close(stepper.alpha(), 0.5);
    assert!(stepper.accumulate(-1.0).is_err());
    close(stepper.backlog(), 0.125);
    stepper.accumulate(0.125)?;
    assert_eq!(stepper.next_step()?.unwrap().index, 3);
    stepper.reset();
    close(stepper.clock().elapsed(), 0.0);
    Ok(())
}

#[test]
fn decimal_frame_time_does_not_lose_a_tick_to_roundoff() -> Result<()> {
    let mut stepper = FixedStepper::new(0.1)?;
    stepper.accumulate(0.3)?;
    let mut count = 0;
    while stepper.next_step()?.is_some() {
        count += 1;
    }
    assert_eq!(count, 3);
    close(stepper.backlog(), 0.0);
    stepper.accumulate(0.099)?;
    assert!(stepper.next_step()?.is_none());
    Ok(())
}

#[test]
fn numeric_capacity_limits_production_and_trade_before_mutation() -> Result<()> {
    let mut goods = Stock::new(10.0)?;
    let mut buyer = Stock::new(0.0)?;
    let mut payment = Stock::new(10.0)?;
    let mut seller = Stock::new(f64::MAX)?;
    close(
        trade(&mut goods, &mut buyer, &mut payment, &mut seller, 1.0, 1.0)?,
        0.0,
    );
    close(goods.val(), 10.0);
    close(payment.val(), 10.0);
    let recipe = Recipe::new(&[1.0], &[1.0], 1.0)?;
    close(recipe.plan(&[&goods], &[&seller], 1.0)?.completed, 0.0);
    close(
        recipe
            .execute(&mut [&mut goods], &mut [&mut seller], 1.0)?
            .completed,
        0.0,
    );
    close(goods.val(), 10.0);
    Ok(())
}

#[test]
fn events_are_stable_cancelable_and_driven_by_explicit_time() -> Result<()> {
    let mut queue = EventQueue::new();
    queue.schedule_at(2.0, "first")?;
    let canceled = queue.schedule_at(1.0, "cancel")?;
    queue.schedule_at(2.0, "second")?;
    assert!(queue.cancel(canceled));
    assert!(!queue.cancel(canceled));
    assert!(queue.advance_to(1.5)?.is_empty());
    let due = queue.advance_to(2.0)?;
    assert_eq!(
        due.iter().map(|x| x.event).collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    assert!(queue.schedule_at(1.0, "past").is_err());
    queue.schedule_after(0.5, "next")?;
    close(queue.pop_next().unwrap().at, 2.5);
    close(queue.now(), 2.5);
    assert!(queue.advance_to(2.0).is_err());
    Ok(())
}

#[test]
fn rng_matches_known_splitmix64_sequence_and_restores_state() {
    let mut rng = Rng::new(0);
    assert_eq!(rng.next_u64(), 0xe220a8397b1dcdaf);
    assert_eq!(rng.next_u64(), 0x6e789e6aa1b965f4);
    let mut restored = Rng::new(rng.state());
    for _ in 0..100 {
        assert_eq!(rng.next_u64(), restored.next_u64());
    }
}

#[test]
fn random_sampling_handles_empty_zero_weight_and_invalid_domains() -> Result<()> {
    let mut rng = Rng::new(55);
    assert_eq!(rng.index(0), None);
    assert_eq!(rng.weighted_index(&[0.0, 0.0])?, None);
    for _ in 0..1000 {
        assert_eq!(rng.weighted_index(&[0.0, 10.0, 0.0])?, Some(1));
        assert!(rng.index(7).unwrap() < 7);
        let value = rng.range(-100.0, 0.0)?;
        assert!((-100.0..0.0).contains(&value));
        assert!(rng.normal(0.0, 1.0)?.is_finite());
    }
    assert!(rng.weighted_index(&[-1.0]).is_err());
    assert!(rng.chance(f64::NAN).is_err());
    assert!(rng.chance(1.0)?);
    assert!(!rng.chance(0.0)?);
    let mut items = [1, 2, 3, 4, 5];
    rng.shuffle(&mut items);
    items.sort();
    assert_eq!(items, [1, 2, 3, 4, 5]);
    Ok(())
}

#[test]
fn online_stats_merge_matches_single_pass_and_window_evicts() -> Result<()> {
    let mut all = RunningStats::new();
    let mut a = RunningStats::new();
    let mut b = RunningStats::new();
    for value in [1.0, 2.0, 3.0, 4.0] {
        all.push(value)?;
    }
    for value in [1.0, 2.0] {
        a.push(value)?;
    }
    for value in [3.0, 4.0] {
        b.push(value)?;
    }
    a.merge(&b)?;
    close(a.mean().unwrap(), 2.5);
    close(a.variance().unwrap(), 1.25);
    close(a.sample_variance().unwrap(), all.sample_variance().unwrap());
    let before = a;
    assert!(a.push(f64::NAN).is_err());
    assert_eq!(a, before);
    let mut window = Window::new(2)?;
    assert_eq!(window.mean(), None);
    assert_eq!(window.push(2.0)?, None);
    window.push(4.0)?;
    assert_eq!(window.push(6.0)?, Some(2.0));
    close(window.mean().unwrap(), 5.0);
    assert_eq!(window.values().copied().collect::<Vec<_>>(), vec![4.0, 6.0]);
    Ok(())
}

#[test]
fn feedback_helpers_have_explicit_half_life_hysteresis_and_delay() -> Result<()> {
    let mut smoother = Smoother::new(0.0, 2.0)?;
    close(smoother.update(10.0, 2.0)?, 5.0);
    let mut threshold = Threshold::new(2.0, 5.0)?;
    assert!(!threshold.update(3.0)?);
    assert!(threshold.update(5.0)?);
    assert!(threshold.update(3.0)?);
    assert!(!threshold.update(2.0)?);
    let mut delay = Delay::new(2, 0);
    assert_eq!(delay.push(10), 0);
    assert_eq!(delay.push(20), 0);
    assert_eq!(delay.push(30), 10);
    assert_eq!(Delay::new(0, 0).push(1), 1);
    Ok(())
}

#[test]
fn response_curves_handle_saturation_and_stable_growth() {
    close(diminishing(10.0, 10.0), 0.5);
    close(s_curve(0.0, 0.0, 1.0), 0.5);
    close(s_curve(1e300, 0.0, 10.0), 1.0);
    close(s_curve(-1e300, 0.0, 10.0), 0.0);
    close(smoothstep(0.0, 10.0, 5.0), 0.5);
    // reversed edges fall smoothly instead of stepping
    close(smoothstep(10.0, 0.0, 10.0), 0.0);
    close(smoothstep(10.0, 0.0, 5.0), 0.5);
    close(smoothstep(10.0, 0.0, 0.0), 1.0);
    close(smoothstep(10.0, 0.0, 2.5), smoothstep(0.0, 10.0, 7.5));
    // zero width is a step at the edge
    close(smoothstep(3.0, 3.0, 2.9), 0.0);
    close(smoothstep(3.0, 3.0, 3.0), 1.0);
    let a = logistic_next(20.0, 100.0, 0.5, 2.0);
    let b = logistic_next(logistic_next(20.0, 100.0, 0.5, 1.0), 100.0, 0.5, 1.0);
    close(a, b);
    assert!(a > 20.0 && a < 100.0);
}

fn replay(seed: u64) -> Result<Vec<(u64, f64, f64)>> {
    let mut clock = Clock::new(0.125)?;
    let mut rng = Rng::new(seed);
    let mut source = Stock::new(100.0)?;
    let mut output = Stock::bounded(0.0, 50.0, 0.0)?;
    let recipe = Recipe::new(&[2.0], &[1.0], 0.9)?;
    let mut events = EventQueue::new();
    events.schedule_at(1.0, 20.0)?;
    let mut history = Vec::new();
    for _ in 0..40 {
        let step = clock.advance()?;
        for event in events.advance_to(step.elapsed)? {
            source.deposit(event.event);
        }
        source.inflow(rng.range(0.0, 1.0)?);
        source.integrate(step.dt)?;
        recipe.execute(&mut [&mut source], &mut [&mut output], step.dt)?;
        history.push((step.index, source.val(), output.val()));
    }
    Ok(history)
}

#[test]
fn composed_simulation_replays_from_seed_and_inputs() -> Result<()> {
    assert_eq!(replay(1836)?, replay(1836)?);
    assert_ne!(replay(1836)?, replay(1837)?);
    Ok(())
}

#[cfg(feature = "fixed")]
#[test]
fn fixed_transfers_conserve_every_bit() -> Result<()> {
    use fluxionyx::fixed::{Amount, FixedStock};
    let mut source = FixedStock::new(Amount::from_num(10), Amount::from_num(20))?;
    let mut target = FixedStock::new(Amount::ZERO, Amount::from_num(5))?;
    let before = source.val().to_bits() + target.val().to_bits();
    let moved = source.flow_to(&mut target, Amount::from_num(12));
    assert_eq!(moved, Amount::from_num(5));
    assert_eq!(source.val().to_bits() + target.val().to_bits(), before);
    source.flow_to(&mut target, Amount::from_num(-1));
    assert_eq!(source.val().to_bits() + target.val().to_bits(), before);
    Ok(())
}
