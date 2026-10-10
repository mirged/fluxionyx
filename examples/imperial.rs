use fluxionyx::prelude::*;

fn main() -> fluxionyx::Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              ĐẠI NAM — IMPERIAL SIMULATION                 ║");
    println!("║                    Campaign Year: 1836                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // ============================================================
    // 1. STATE CAPACITY
    // ============================================================

    let mut sovereignty = Stat::new(100.0);

    sovereignty.add("court_factionalism", -6.0);
    sovereignty.add("southern_smuggling", -4.0);
    sovereignty.add("foreign_trade_concessions", -8.0);
    sovereignty.mult("imperial_administration", 0.94);

    println!("\n┌─ IMPERIAL COURT ─────────────────────────────────────────────");
    println!(
        "│ Sovereignty .............. {:>6.1} / 100",
        sovereignty.val()
    );
    println!(
        "│ Mandate status ........... {}",
        match sovereignty.val() {
            x if x >= 90.0 => "UNSHAKEN",
            x if x >= 75.0 => "STABLE",
            x if x >= 50.0 => "STRAINED",
            _ => "WE ARE SO COOKED",
        }
    );
    println!("│ Foreign pressure ......... RISING");
    println!("└──────────────────────────────────────────────────────────────");

    // ============================================================
    // 2. IMPERIAL TREASURY
    // ============================================================

    let mut treasury = Stock::bounded(0.0, 2_000_000.0, 1_240_000.0)?;

    let mut provincial_treasury = Stock::bounded(0.0, 500_000.0, 82_000.0)?;

    // Huế transfers emergency funds south after crop failures.
    let relief_funds = treasury.flow_to(&mut provincial_treasury, 35_000.0);

    // Administrative losses over the year.
    let corruption_loss = treasury.leak(320.0, 52.0);

    // Coin clipping, transport loss and accounting errors.
    // A 230.7-year half-life is approximately 0.3% loss per year.
    let monetary_loss = treasury.evaporate(230.7, 1.0);

    println!("\n┌─ MINISTRY OF REVENUE ────────────────────────────────────────");
    println!("│ Opening reserves ......... 1,240,000 quan");
    println!("│ Southern relief .......... {:>9.0} quan", relief_funds);
    println!("│ 'Administrative loss' .... {:>9.0} quan", corruption_loss);
    println!("│ Monetary degradation ..... {:>9.0} quan", monetary_loss);
    println!("│");
    println!("│ Imperial treasury ........ {:>9.0} quan", treasury.val());
    println!(
        "│ Reserve capacity ......... {:>8.1}%",
        treasury.ratio().unwrap() * 100.0
    );

    if corruption_loss > 10_000.0 {
        println!("│");
        println!("│ ⚠ Ministry explanation: \"accounting irregularities\"");
        println!("│   Emperor's explanation:  \"find whoever stole it\"");
    }

    println!("└──────────────────────────────────────────────────────────────");

    // ============================================================
    // 3. STATE ARSENAL
    // ============================================================

    let mut iron = Stock::bounded(0.0, 20_000.0, 8_400.0)?;
    let mut charcoal = Stock::bounded(0.0, 10_000.0, 3_100.0)?;
    let mut muskets = Stock::bounded(0.0, 10_000.0, 420.0)?;

    // Simplified arsenal production:
    //
    //   4 iron + 1.5 charcoal -> 1 musket
    //
    // Workshops operate at only 73% efficiency because of
    // inconsistent metal quality and artisanal production.

    let mut inputs = [(&mut iron, 4.0), (&mut charcoal, 1.5)];

    let mut outputs = [(&mut muskets, 1.0)];

    let production = convert_lossy(&mut inputs, &mut outputs, 0.73, 1_000.0);

    println!("\n┌─ HUẾ IMPERIAL ARSENAL ───────────────────────────────────────");
    println!("│ Production order ......... 1,000 muskets");
    println!("│ Effective batches ........ {:>8.1}", production);
    println!(
        "│ Arsenal stock ............ {:>8.0} muskets",
        muskets.val()
    );
    println!("│ Iron remaining ........... {:>8.0}", iron.val());
    println!("│ Charcoal remaining ....... {:>8.0}", charcoal.val());
    println!("│");
    println!("│ Workshop efficiency ...... 73%");
    println!("│ European inspector ....... \"concerning\"");
    println!("│ Court inspector .......... \"acceptable\"");
    println!("└──────────────────────────────────────────────────────────────");

    // ============================================================
    // 4. RICE MARKET
    // ============================================================

    let normal_rice_price = 12.0;

    let rice_supply = 1_420_000.0;
    let rice_demand = 1_760_000.0;

    let rice_price = elastic(rice_demand, rice_supply, normal_rice_price, 1.25);

    let unrest = diminishing(
        (rice_price / normal_rice_price - 1.0).max(0.0) * 100.0,
        25.0,
    );

    let unrest = soft_cap(unrest * 100.0, 100.0, 18.0);

    println!("\n┌─ MEKONG RICE MARKET ─────────────────────────────────────────");
    println!("│ Supply ................... {:>10.0}", rice_supply);
    println!("│ Demand ................... {:>10.0}", rice_demand);
    println!(
        "│ Normal price ............. {:>8.2} quan",
        normal_rice_price
    );
    println!("│ Market price ............. {:>8.2} quan", rice_price);
    println!("│");
    println!("│ Popular unrest ........... {:>8.1} / 100", unrest);

    if rice_price > normal_rice_price * 1.20 {
        println!("│");
        println!("│ ⚠ Rice prices are climbing.");
        println!("│   Local officials recommend opening state granaries.");
    }

    println!("└──────────────────────────────────────────────────────────────");

    // ============================================================
    // 5. POPULATION
    // ============================================================

    let mut population = Cohort::new(16, 5.0);

    // Approximate initial cohort seeding through births + ageing.
    //
    // Not intended as historical demographic data; this is a
    // simulation-engine stress scenario.

    for generation in [
        720_000.0, 690_000.0, 660_000.0, 625_000.0, 590_000.0, 540_000.0, 490_000.0, 430_000.0,
    ] {
        population.births(generation);
        population.advance(5.0);
    }

    population.births(760_000.0);

    let deaths = population.cull_rate(
        |age| {
            if age < 5 {
                0.035
            } else if age >= 60 {
                0.075
            } else {
                0.012
            }
        },
        1.0,
    );

    let workers = population.working_slice(15.0, 60.0);

    println!("\n┌─ IMPERIAL CENSUS OFFICE ─────────────────────────────────────");
    println!("│ Recorded population ...... {:>10.0}", population.total());
    println!("│ Working-age population ... {:>10.0}", workers);
    println!("│ Annual deaths ............ {:>10.0}", deaths);
    println!("│");
    println!("│ Census confidence ........ LOW");
    println!("│ Remote villages counted .. \"probably\"");
    println!("└──────────────────────────────────────────────────────────────");

    // ============================================================
    // 6. FOREIGN INFLUENCE
    // ============================================================

    let mut foreign_influence = Field2D::new(8, 5, 0.0);

    // Coastal commercial footholds.
    foreign_influence.set(0, 1, 85.0);
    foreign_influence.set(0, 3, 60.0);

    // Trade, missionaries and diplomatic contact slowly penetrate
    // inland, while terrain and administrative resistance reduce it.

    foreign_influence.diffuse(0.18, 3.0);
    foreign_influence.decay_all(0.88);

    println!("\n┌─ FOREIGN AFFAIRS BUREAU ─────────────────────────────────────");
    println!(
        "│ Coastal influence ........ {:>7.2}",
        foreign_influence.get(0, 1)
    );
    println!(
        "│ Interior influence ....... {:>7.2}",
        foreign_influence.get(3, 2)
    );
    println!(
        "│ Remote interior .......... {:>7.2}",
        foreign_influence.get(7, 4)
    );
    println!("│");
    println!("│ British merchants ........ ACTIVE");
    println!("│ French missionaries ...... ACTIVE");
    println!("│ Qing influence ........... SIGNIFICANT");
    println!("│ Court response ........... SUSPICIOUS STARING");
    println!("└──────────────────────────────────────────────────────────────");

    // ============================================================
    // 7. POPULATION GROWTH PROJECTION
    // ============================================================

    let current_population = population.total();

    let carrying_capacity = 12_000_000.0;

    let five_year_growth = logistic(current_population, carrying_capacity, 0.018, 5.0);

    println!("\n┌─ FIVE-YEAR IMPERIAL PROJECTION ───────────────────────────────");
    println!("│ Current population ....... {:>10.0}", current_population);
    println!("│ Estimated growth ......... +{:>9.0}", five_year_growth);
    println!("│ Carrying capacity ........ {:>10.0}", carrying_capacity);
    println!("└──────────────────────────────────────────────────────────────");

    // ============================================================
    // FINAL REPORT
    // ============================================================

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                   IMPERIAL REPORT — 1836                   ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║ Treasury             {:>12.0} quan                  ║",
        treasury.val()
    );
    println!(
        "║ Sovereignty          {:>12.1} / 100                 ║",
        sovereignty.val()
    );
    println!(
        "║ Rice Price           {:>12.2} quan                  ║",
        rice_price
    );
    println!(
        "║ Population           {:>12.0}                       ║",
        population.total()
    );
    println!(
        "║ Working Population   {:>12.0}                       ║",
        workers
    );
    println!(
        "║ Muskets              {:>12.0}                       ║",
        muskets.val()
    );
    println!("╠══════════════════════════════════════════════════════════════╣");

    if unrest > 60.0 {
        println!("║ STATUS: THE PEASANTS ARE BEGINNING TO HAVE OPINIONS         ║");
    } else if sovereignty.val() < 60.0 {
        println!("║ STATUS: FOREIGN MINISTERS ARE GETTING SUSPICIOUSLY CONFIDENT║");
    } else {
        println!("║ STATUS: THE EMPIRE CONTINUES TO EXIST. GREAT SUCCESS.       ║");
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    println!();
    println!("Simulation tick complete.");
    println!("No dynasties collapsed during this tick.");

    Ok(())
}
