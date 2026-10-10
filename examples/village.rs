//! A small economy with timed policy, weather, production, and observations.
use fluxionyx::prelude::*;

#[derive(Debug)]
enum Event {
    Drought,
    Rain,
}

fn main() -> fluxionyx::Result<()> {
    let mut clock = Clock::new(0.25)?; // All rates are per day.
    let mut rng = Rng::new(1836);
    let mut events = EventQueue::new();
    events.schedule_at(3.0, Event::Drought)?;
    events.schedule_at(6.0, Event::Rain)?;
    let mut water = Stock::bounded(0.0, 200.0, 150.0)?;
    let mut grain = Stock::bounded(0.0, 300.0, 80.0)?;
    let mut food = Stock::bounded(0.0, 120.0, 20.0)?;
    let mill = Recipe::new(&[2.0], &[1.0], 0.9)?;
    let mut harvest = Stat::new(12.0);
    harvest.percent("irrigation", 0.2);
    let mut shortage = Threshold::new(0.2, 0.6)?;
    let mut price = Smoother::new(1.0, 1.0)?;
    let mut prices = RunningStats::new();
    let mut history = Window::new(8)?;
    let mut rainfall = 10.0;

    for _ in 0..40 {
        let step = clock.advance()?;
        for event in events.advance_to(step.elapsed)? {
            match event.event {
                Event::Drought => rainfall = 1.0,
                Event::Rain => rainfall = 12.0,
            }
        }
        water.inflow(rainfall * rng.range(0.8, 1.2)?);
        water.outflow(8.0);
        water.integrate(step.dt)?;
        grain.inflow(harvest.val() * diminishing(water.val(), 30.0));
        grain.integrate(step.dt)?;
        let report = mill.execute(&mut [&mut grain], &mut [&mut food], 5.0 * step.dt)?;
        let demand = 4.0 * step.dt;
        let consumed = food.withdraw(demand);
        let unmet = 1.0 - consumed / demand;
        let alarm = shortage.update(unmet)?;
        let market_price = elastic(demand, report.produced.max(0.1), 1.0, 0.7);
        price.update(market_price, step.dt)?;
        prices.push(price.val())?;
        history.push(food.val())?;
        if step.index % 4 == 0 {
            println!(
                "day {:2.0}: water={:6.1}, grain={:6.1}, food={:5.1}, price={:.2}, shortage={alarm}",
                step.elapsed,
                water.val(),
                grain.val(),
                food.val(),
                price.val()
            );
        }
    }
    println!(
        "Mean price: {:.2}; food in last two days: {:.1}",
        prices.mean().unwrap_or(0.0),
        history.mean().unwrap_or(0.0)
    );
    Ok(())
}
