use evolving_market::agents::{BuyerAgent, SellerAgent};
use evolving_market::coordinator::MarketCoordinator;
use evolving_market::{Session, loyalty_concentration};
use rand::{Rng, SeedableRng};
use std::fs::File;
use std::io::Write as _;

/// Minimal model configuration
const N_BUYERS: usize = 100;
const N_SELLERS: usize = 10;
const N_DAYS: usize = 1000; // Start with 1000 days for testing
const BUYER_P_OUT: usize = 15; // Homogeneous buyers
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
}

fn main() {
    println!("Evolving Market Structure - Minimal Model");
    println!("==========================================");
    println!("Configuration:");
    println!("  Buyers: {}", N_BUYERS);
    println!("  Sellers: {}", N_SELLERS);
    println!("  Days: {}", N_DAYS);
    println!("  Buyer p_out: {}", BUYER_P_OUT);
    println!("  Seller p_in: {}", SELLER_P_IN);
    println!("  Initial stock per seller: {}", SELLER_INITIAL_STOCK);
    println!();

    // Create RNG for reproducibility
    let mut main_rng = rand::rngs::StdRng::seed_from_u64(42);

    // Create buyers (homogeneous)
    let mut buyers: Vec<BuyerAgent> = (0..N_BUYERS)
        .map(|id| {
            let seed = main_rng.random();
            BuyerAgent::new(id, BUYER_P_OUT, N_SELLERS, MAX_PRICE, seed)
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

        // Collect statistics
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

        let stats = if !prices.is_empty() {
            DayStats {
                day,
                avg_price: prices.iter().sum::<usize>() as f64 / prices.len() as f64,
                min_price: *prices.iter().min().unwrap(),
                max_price: *prices.iter().max().unwrap(),
                n_transactions,
                n_denied,
                avg_loyalty_concentration: coordinator.average_loyalty_concentration(),
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
            }
        };

        daily_stats.push(stats.clone());

        // Print progress every 100 days
        if (day + 1) % 100 == 0 {
            // Calculate average beta across sellers
            let avg_beta = sellers.iter().map(|s| s.beta as f64).sum::<f64>() / sellers.len() as f64;

            println!(
                "Day {}: avg_price={:.2}, transactions={}, denied={}, loyalty={:.3}, avg_beta={:.1}",
                day + 1,
                stats.avg_price,
                stats.n_transactions,
                stats.n_denied,
                stats.avg_loyalty_concentration,
                avg_beta
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
}

fn write_csv(stats: &[DayStats]) -> std::io::Result<()> {
    let mut file = File::create("market_stats.csv")?;

    writeln!(file, "day,avg_price,min_price,max_price,n_transactions,n_denied,avg_loyalty")?;

    for s in stats {
        writeln!(
            file,
            "{},{:.2},{},{},{},{},{:.4}",
            s.day, s.avg_price, s.min_price, s.max_price, s.n_transactions, s.n_denied, s.avg_loyalty_concentration
        )?;
    }

    Ok(())
}
