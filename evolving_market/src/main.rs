use evolving_market::agents::{BuyerAgent, SellerAgent};
use evolving_market::coordinator::MarketCoordinator;
use evolving_market::{BuyerType, Session, loyalty_concentration};
use rand::{Rng, SeedableRng};
use std::fs::File;
use std::io::Write as _;

/// Model configuration with heterogeneous buyers
const N_BUYERS: usize = 100;
const N_SELLERS: usize = 10;
const N_DAYS: usize = 1000;
const SELLER_P_IN: usize = 9;
const SELLER_INITIAL_STOCK: usize = 15; // 10 sellers × 15 = 150 units for 100 buyers
const MAX_PRICE: usize = 20;
const ALPHA: f64 = 0.25; // Loyalty decay parameter
const LEARNING_RATE: f64 = 0.05;

/// Daily statistics for output
#[derive(Debug, Clone)]
struct DayStats {
    day: usize,
    avg_price: f64,
    min_price: usize,
    max_price: usize,
    n_transactions: usize,
    n_denied: usize,
    avg_loyalty_concentration: f64,
    // Per-type statistics
    avg_price_low: f64,
    avg_price_medium: f64,
    avg_price_high: f64,
    trans_rate_low: f64,
    trans_rate_medium: f64,
    trans_rate_high: f64,
}

fn main() {
    println!("Evolving Market Structure - Heterogeneous Buyers Model");
    println!("========================================================");
    println!("Configuration:");
    println!("  Buyers: {} (33% Low/34% Medium/33% High valuation)", N_BUYERS);
    println!("  Sellers: {}", N_SELLERS);
    println!("  Days: {}", N_DAYS);
    println!("  Buyer valuations: Low=12, Medium=15, High=18");
    println!("  Seller p_in: {}", SELLER_P_IN);
    println!("  Initial stock per seller: {}", SELLER_INITIAL_STOCK);
    println!();

    // Create RNG for reproducibility
    let mut main_rng = rand::rngs::StdRng::seed_from_u64(42);

    // Create buyers (heterogeneous: 33% low, 34% medium, 33% high)
    let mut buyers: Vec<BuyerAgent> = (0..N_BUYERS)
        .map(|id| {
            let buyer_type = if id < 33 {
                BuyerType::Low
            } else if id < 67 {
                BuyerType::Medium
            } else {
                BuyerType::High
            };
            let seed = main_rng.random();
            BuyerAgent::new(id, buyer_type, N_SELLERS, MAX_PRICE, seed)
        })
        .collect();

    // Create sellers
    let mut sellers: Vec<SellerAgent> = (0..N_SELLERS)
        .map(|id| {
            let seed = main_rng.random();
            SellerAgent::new(id, SELLER_P_IN, SELLER_INITIAL_STOCK, MAX_PRICE, seed)
        })
        .collect();

    // Create market coordinator
    let mut coordinator = MarketCoordinator::new(N_BUYERS, N_SELLERS, ALPHA);

    // Statistics collection
    let mut daily_stats = Vec::new();

    // Main simulation loop
    for day in 0..N_DAYS {
        // Reset daily state
        coordinator.reset_session();
        for seller in &mut sellers {
            seller.reset_daily_state();
        }
        for buyer in &mut buyers {
            buyer.reset_daily_state();
        }

        // Phase 1: Buyers choose sellers
        for buyer in &mut buyers {
            let seller_id = buyer.choose_seller();
            coordinator.record_buyer_choice(buyer.id, seller_id);
        }

        // Phase 2: Sellers choose beta parameter for queue handling
        for seller in &mut sellers {
            seller.choose_beta();
        }

        // Phase 3: Form queues
        coordinator.form_queues();

        // Phase 4: Process queues (sellers offer prices, buyers respond)
        let _transaction_events = coordinator.process_queues(
            &mut sellers,
            &mut buyers,
            day,
            Session::Morning,
        );

        // Phase 5: Update learning (both buyers and sellers)
        for buyer in &mut buyers {
            buyer.update_strengths(LEARNING_RATE);
        }
        for seller in &mut sellers {
            seller.update_strengths(LEARNING_RATE);
        }

        // Phase 6: Update loyalty
        coordinator.update_loyalty();

        // Collect statistics (overall and per-type)
        let prices: Vec<usize> = buyers
            .iter()
            .filter_map(|b| {
                if b.transaction_completed {
                    b.price_offered
                } else {
                    None
                }
            })
            .collect();

        let n_transactions = buyers.iter().filter(|b| b.transaction_completed).count();
        let n_denied = buyers.iter().filter(|b| b.denied_service).count();

        // Per-type statistics
        let compute_type_stats = |buyer_type: BuyerType| {
            let type_buyers: Vec<_> = buyers.iter().filter(|b| b.buyer_type == buyer_type).collect();
            let type_prices: Vec<usize> = type_buyers
                .iter()
                .filter_map(|b| if b.transaction_completed { b.price_offered } else { None })
                .collect();
            let type_trans = type_buyers.iter().filter(|b| b.transaction_completed).count();
            let avg_price = if !type_prices.is_empty() {
                type_prices.iter().sum::<usize>() as f64 / type_prices.len() as f64
            } else {
                0.0
            };
            let trans_rate = type_trans as f64 / type_buyers.len() as f64;
            (avg_price, trans_rate)
        };

        let (avg_price_low, trans_rate_low) = compute_type_stats(BuyerType::Low);
        let (avg_price_medium, trans_rate_medium) = compute_type_stats(BuyerType::Medium);
        let (avg_price_high, trans_rate_high) = compute_type_stats(BuyerType::High);

        let stats = if !prices.is_empty() {
            DayStats {
                day,
                avg_price: prices.iter().sum::<usize>() as f64 / prices.len() as f64,
                min_price: *prices.iter().min().unwrap(),
                max_price: *prices.iter().max().unwrap(),
                n_transactions,
                n_denied,
                avg_loyalty_concentration: coordinator.average_loyalty_concentration(),
                avg_price_low,
                avg_price_medium,
                avg_price_high,
                trans_rate_low,
                trans_rate_medium,
                trans_rate_high,
            }
        } else {
            DayStats {
                day,
                avg_price: 0.0,
                min_price: 0,
                max_price: 0,
                n_transactions,
                n_denied,
                avg_loyalty_concentration: coordinator.average_loyalty_concentration(),
                avg_price_low,
                avg_price_medium,
                avg_price_high,
                trans_rate_low,
                trans_rate_medium,
                trans_rate_high,
            }
        };

        daily_stats.push(stats.clone());

        // Print progress every 100 days
        if (day + 1) % 100 == 0 {
            // Calculate average beta across sellers
            let avg_beta = sellers.iter().map(|s| s.beta as f64).sum::<f64>() / sellers.len() as f64;

            println!(
                "Day {}: avg_price={:.2}, loyalty={:.3}, β={:.1} | Prices: L={:.2} M={:.2} H={:.2} | Trans: L={:.0}% M={:.0}% H={:.0}%",
                day + 1,
                stats.avg_price,
                stats.avg_loyalty_concentration,
                avg_beta,
                stats.avg_price_low,
                stats.avg_price_medium,
                stats.avg_price_high,
                stats.trans_rate_low * 100.0,
                stats.trans_rate_medium * 100.0,
                stats.trans_rate_high * 100.0
            );
        }
    }

    // Write results to CSV
    write_csv(&daily_stats).expect("Failed to write CSV");
    println!("\nResults written to market_stats.csv");

    // Print final summary
    let final_stats = daily_stats.last().unwrap();
    println!("\nFinal Statistics (Day {}):", N_DAYS);
    println!("  Average price: {:.2}", final_stats.avg_price);
    println!("  Price range: {} - {}", final_stats.min_price, final_stats.max_price);
    println!("  Transactions: {}", final_stats.n_transactions);
    println!("  Denied service: {}", final_stats.n_denied);
    println!("  Average loyalty concentration: {:.3}", final_stats.avg_loyalty_concentration);

    // Compute loyalty distribution
    let concentrations: Vec<f64> = (0..N_BUYERS)
        .map(|buyer_id| loyalty_concentration(&coordinator.loyalty[buyer_id]))
        .collect();

    let high_loyalty = concentrations.iter().filter(|&&c| c > 0.9).count();
    let medium_loyalty = concentrations.iter().filter(|&&c| c > 0.6 && c <= 0.9).count();
    let low_loyalty = concentrations.iter().filter(|&&c| c <= 0.6).count();

    println!("\nLoyalty Distribution:");
    println!("  High (γ > 0.9): {} buyers ({:.1}%)", high_loyalty, 100.0 * high_loyalty as f64 / N_BUYERS as f64);
    println!("  Medium (0.6 < γ ≤ 0.9): {} buyers ({:.1}%)", medium_loyalty, 100.0 * medium_loyalty as f64 / N_BUYERS as f64);
    println!("  Low (γ ≤ 0.6): {} buyers ({:.1}%)", low_loyalty, 100.0 * low_loyalty as f64 / N_BUYERS as f64);

    println!("\nPrice Discrimination by Buyer Type:");
    println!("  Low valuation  (p_out=12): avg price={:.2}, trans rate={:.0}%",
        final_stats.avg_price_low, final_stats.trans_rate_low * 100.0);
    println!("  Medium valuation (p_out=15): avg price={:.2}, trans rate={:.0}%",
        final_stats.avg_price_medium, final_stats.trans_rate_medium * 100.0);
    println!("  High valuation (p_out=18): avg price={:.2}, trans rate={:.0}%",
        final_stats.avg_price_high, final_stats.trans_rate_high * 100.0);

    let price_spread = final_stats.avg_price_high - final_stats.avg_price_low;
    println!("\nPrice Spread (High - Low): {:.2}", price_spread);
    if price_spread > 0.5 {
        println!("  ✓ Price discrimination detected!");
    } else {
        println!("  ✗ Little price discrimination");
    }
}

fn write_csv(stats: &[DayStats]) -> std::io::Result<()> {
    let mut file = File::create("market_stats.csv")?;

    writeln!(
        file,
        "day,avg_price,min_price,max_price,n_transactions,n_denied,avg_loyalty,\
         price_low,price_medium,price_high,trans_rate_low,trans_rate_medium,trans_rate_high"
    )?;

    for s in stats {
        writeln!(
            file,
            "{},{:.2},{},{},{},{},{:.4},{:.2},{:.2},{:.2},{:.3},{:.3},{:.3}",
            s.day, s.avg_price, s.min_price, s.max_price, s.n_transactions, s.n_denied,
            s.avg_loyalty_concentration,
            s.avg_price_low, s.avg_price_medium, s.avg_price_high,
            s.trans_rate_low, s.trans_rate_medium, s.trans_rate_high
        )?;
    }

    Ok(())
}
