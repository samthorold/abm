//! Data output and serialization for experimental analysis
//!
//! This module provides structured export of simulation results to CSV and JSON formats
//! for analysis in Python (pandas, scipy, matplotlib).
//!
//! ## Time Series Data Availability
//!
//! **Full historical tracking:**
//! - `loss_ratio` - Primary validation metric (cycle detection)
//! - `avg_claim` - Supporting metric for claim dynamics
//!
//! **Final year only:**
//! - `total_premiums`, `total_claims` - Industry aggregates
//! - `avg_price`, `min_price`, `max_price` - Pricing statistics
//! - `herfindahl_index`, `gini_coefficient` - Market concentration
//! - `num_solvent_insurers` - Market structure
//!
//! This design minimizes memory overhead (avoids 9 additional vectors × 100 years)
//! and aligns with DES framework constraints (no shadow state expansion).
//! Market structure analysis uses final state snapshots and per-insurer data.

use crate::{MarketStats, ModelConfig, Stats};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Top-level container for simulation output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOutput {
    pub metadata: SimulationMetadata,
    pub market_timeseries: Vec<MarketTimePoint>,
    pub insurer_snapshots: Vec<InsurerSnapshot>,
    pub cycle_metrics: CycleMetrics,
}

/// Metadata for reproducibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationMetadata {
    pub config: SerializableConfig,
    pub seed: u64,
    pub num_years: usize,
    pub timestamp: String,
    pub git_commit: Option<String>,
}

/// Serializable version of ModelConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableConfig {
    pub risk_loading_factor: f64,
    pub underwriter_smoothing: f64,
    pub distance_cost: f64,
    pub credibility_factor: f64,
    pub ewma_smoothing: f64,
    pub claim_frequency: f64,
    pub gamma_mean: f64,
    pub gamma_std: f64,
    pub num_insurers: usize,
    pub num_customers: usize,
    pub initial_capital: f64,
    pub leverage_ratio: f64,
}

impl From<&ModelConfig> for SerializableConfig {
    fn from(config: &ModelConfig) -> Self {
        SerializableConfig {
            risk_loading_factor: config.risk_loading_factor,
            underwriter_smoothing: config.underwriter_smoothing,
            distance_cost: config.distance_cost,
            credibility_factor: config.credibility_factor,
            ewma_smoothing: config.ewma_smoothing,
            claim_frequency: config.claim_frequency,
            gamma_mean: config.gamma_mean,
            gamma_std: config.gamma_std,
            num_insurers: config.num_insurers,
            num_customers: config.num_customers,
            initial_capital: config.initial_capital,
            leverage_ratio: config.leverage_ratio,
        }
    }
}

/// Single time point in market time series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTimePoint {
    pub year: usize,
    pub loss_ratio: f64,
    pub avg_claim: f64,
    pub total_premiums: f64,
    pub total_claims: f64,
    pub avg_price: f64,
    pub min_price: f64,
    pub max_price: f64,
    pub num_solvent_insurers: usize,
    pub herfindahl_index: f64,
    pub gini_coefficient: f64,
}

/// Insurer state snapshot at a specific year
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsurerSnapshot {
    pub year: usize,
    pub insurer_id: usize,
    pub capital: f64,
    pub market_share: f64,
    pub price: f64,
    pub markup: f64,
    pub loss_ratio: f64,
    pub num_customers: usize,
    pub is_solvent: bool,
}

/// Aggregate cycle metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleMetrics {
    pub has_cycles: bool,
    pub cycle_period: Option<f64>,
    pub dominant_frequency: Option<f64>,
    pub mean_loss_ratio: f64,
    pub std_loss_ratio: f64,
    pub cycle_amplitude: f64,
    pub ar2_coefficients: Option<(f64, f64, f64)>, // (a0, a1, a2)
    pub meets_cycle_conditions: Option<bool>,
}

impl SimulationOutput {
    /// Convert Stats vector to structured output
    ///
    /// # Arguments
    /// * `all_stats` - Stats from all agents after simulation
    /// * `config` - Model configuration
    /// * `seed` - Random seed used
    /// * `num_years` - Number of years simulated
    pub fn from_stats(
        all_stats: Vec<Stats>,
        config: &ModelConfig,
        seed: u64,
        num_years: usize,
    ) -> Self {
        // Extract market stats
        let market_stats = all_stats
            .iter()
            .find_map(|s| {
                if let Stats::Market(m) = s {
                    Some(m)
                } else {
                    None
                }
            })
            .expect("No market stats found");

        // Build market time series from loss_ratio_history
        let mut market_timeseries = Vec::new();
        for (year_idx, &loss_ratio) in market_stats.loss_ratio_history.iter().enumerate() {
            // Note: year_idx + 1 because simulation starts at year 1
            let year = year_idx + 1;

            // For historical data, we only have limited info per year
            // This is a simplified version - in reality, we'd need to track more per-year data
            market_timeseries.push(MarketTimePoint {
                year,
                loss_ratio,
                avg_claim: market_stats
                    .avg_claim_history
                    .get(year_idx)
                    .copied()
                    .unwrap_or(0.0),
                total_premiums: 0.0, // Would need year-by-year tracking
                total_claims: 0.0,   // Would need year-by-year tracking
                avg_price: 0.0,      // Would need year-by-year tracking
                min_price: 0.0,
                max_price: 0.0,
                num_solvent_insurers: 0,
                herfindahl_index: 0.0,
                gini_coefficient: 0.0,
            });
        }

        // Extract insurer snapshots (final year only)
        let insurer_snapshots: Vec<InsurerSnapshot> = all_stats
            .iter()
            .filter_map(|s| {
                if let Stats::Insurer(ins) = s {
                    Some(InsurerSnapshot {
                        year: num_years,
                        insurer_id: ins.insurer_id,
                        capital: ins.capital,
                        market_share: ins.market_share,
                        price: ins.current_market_price,
                        markup: ins.current_markup,
                        loss_ratio: ins.loss_ratio,
                        num_customers: ins.num_customers,
                        is_solvent: ins.is_solvent(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Calculate market concentration from final insurer snapshots
        let total_customers: usize = insurer_snapshots
            .iter()
            .filter(|s| s.is_solvent)
            .map(|s| s.num_customers)
            .sum();

        let market_shares: Vec<f64> = if total_customers > 0 {
            insurer_snapshots
                .iter()
                .filter(|s| s.is_solvent)
                .map(|s| s.num_customers as f64 / total_customers as f64)
                .collect()
        } else {
            vec![]
        };

        // Use final year stats for most recent data
        if let Some(last_point) = market_timeseries.last_mut() {
            last_point.total_premiums = market_stats.total_premiums;
            last_point.total_claims = market_stats.total_claims;
            last_point.avg_price = market_stats.avg_price;
            last_point.min_price = market_stats.min_price;
            last_point.max_price = market_stats.max_price;
            last_point.num_solvent_insurers = market_stats.num_solvent_insurers;
            last_point.herfindahl_index = MarketStats::calculate_herfindahl(&market_shares);
            last_point.gini_coefficient = MarketStats::calculate_gini(&market_shares);
        }

        // Calculate cycle metrics
        let cycle_metrics = CycleMetrics {
            has_cycles: market_stats.has_cycles(),
            cycle_period: market_stats.cycle_period(),
            dominant_frequency: market_stats.dominant_frequency(),
            mean_loss_ratio: market_stats.mean_loss_ratio(),
            std_loss_ratio: market_stats.std_loss_ratio(),
            cycle_amplitude: market_stats.cycle_amplitude(),
            ar2_coefficients: market_stats.fit_ar2(),
            meets_cycle_conditions: market_stats.check_cycle_conditions(),
        };

        // Get git commit if available
        let git_commit = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        // Get current timestamp
        let timestamp = chrono::Utc::now().to_rfc3339();

        SimulationOutput {
            metadata: SimulationMetadata {
                config: SerializableConfig::from(config),
                seed,
                num_years,
                timestamp,
                git_commit,
            },
            market_timeseries,
            insurer_snapshots,
            cycle_metrics,
        }
    }

    /// Write market time series to CSV
    pub fn write_market_csv<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut wtr = csv::Writer::from_path(path)?;

        // Write header
        wtr.write_record([
            "year",
            "loss_ratio",
            "avg_claim",
            "total_premiums",
            "total_claims",
            "avg_price",
            "min_price",
            "max_price",
            "num_solvent_insurers",
            "herfindahl_index",
            "gini_coefficient",
        ])?;

        // Write data
        for point in &self.market_timeseries {
            wtr.write_record(&[
                point.year.to_string(),
                point.loss_ratio.to_string(),
                point.avg_claim.to_string(),
                point.total_premiums.to_string(),
                point.total_claims.to_string(),
                point.avg_price.to_string(),
                point.min_price.to_string(),
                point.max_price.to_string(),
                point.num_solvent_insurers.to_string(),
                point.herfindahl_index.to_string(),
                point.gini_coefficient.to_string(),
            ])?;
        }

        wtr.flush()?;
        Ok(())
    }

    /// Write insurer snapshots to CSV
    pub fn write_insurer_csv<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut wtr = csv::Writer::from_path(path)?;

        // Write header
        wtr.write_record([
            "year",
            "insurer_id",
            "capital",
            "market_share",
            "price",
            "markup",
            "loss_ratio",
            "num_customers",
            "is_solvent",
        ])?;

        // Write data
        for snapshot in &self.insurer_snapshots {
            wtr.write_record(&[
                snapshot.year.to_string(),
                snapshot.insurer_id.to_string(),
                snapshot.capital.to_string(),
                snapshot.market_share.to_string(),
                snapshot.price.to_string(),
                snapshot.markup.to_string(),
                snapshot.loss_ratio.to_string(),
                snapshot.num_customers.to_string(),
                snapshot.is_solvent.to_string(),
            ])?;
        }

        wtr.flush()?;
        Ok(())
    }

    /// Write summary JSON with metadata and cycle metrics
    pub fn write_summary_json<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Write all outputs to a directory
    ///
    /// Creates:
    /// - market_timeseries.csv
    /// - insurer_snapshots.csv
    /// - summary.json
    pub fn write_all<P: AsRef<Path>>(&self, dir: P) -> Result<(), Box<dyn std::error::Error>> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;

        self.write_market_csv(dir.join("market_timeseries.csv"))?;
        self.write_insurer_csv(dir.join("insurer_snapshots.csv"))?;
        self.write_summary_json(dir.join("summary.json"))?;

        Ok(())
    }
}
