use crate::{
    CombinedMarketStats, Event, MarketSnapshot, Stats, SyndicateTimeSeriesStats, TimeSeriesStats,
};
use des::{Agent, Response};
use std::collections::HashMap;

/// Number of days in a simulated year
const DAYS_PER_YEAR: usize = 365;

/// Collects market statistics over time to build a time series
///
/// Design rationale: This agent exists separately (rather than being folded into
/// TimeGenerator or another agent) because:
/// 1. It represents a distinct real-world entity: regulatory/market oversight that
///    collects and publishes aggregate market statistics
/// 2. It follows single-responsibility principle: only aggregates market data
/// 3. It maintains clean separation of concerns: TimeGenerator manages time flow,
///    MarketStatisticsCollector observes market state
/// 4. It enables modular testing and future extensions (e.g., real-time market alerts)
///
/// Trade-off: Adds one more agent to the simulation, but the observability and
/// modularity benefits outweigh the minimal broadcast overhead.
pub struct MarketStatisticsCollector {
    num_syndicates: usize,
    time_series: TimeSeriesStats,
    syndicate_time_series: SyndicateTimeSeriesStats, // NEW: Per-syndicate time series

    // Temporary storage for collecting stats within a single time period
    current_year: usize,
    current_day: usize,
    pending_reports: HashMap<usize, SyndicateReport>,

    // NEW: Catastrophe tracking for current year
    cat_event_occurred: bool,
    cat_event_total_loss: f64,
}

#[derive(Debug, Clone)]
struct SyndicateReport {
    capital: f64,
    is_insolvent: bool,
    annual_premiums: f64,
    annual_claims: f64,
    num_policies: usize,    // Annual policies written (not cumulative)
    num_claims: usize,      // Annual claims received (not cumulative)
    markup_m_t: f64,        // NEW: Underwriting markup
    uniform_deviation: f64, // NEW: Exposure uniformity metric
}

impl MarketStatisticsCollector {
    pub fn new(num_syndicates: usize) -> Self {
        Self {
            num_syndicates,
            time_series: TimeSeriesStats::new(),
            syndicate_time_series: SyndicateTimeSeriesStats::new(),
            current_year: 0,
            current_day: 0,
            pending_reports: HashMap::new(),
            cat_event_occurred: false,
            cat_event_total_loss: 0.0,
        }
    }

    fn handle_year(&mut self, current_t: usize) {
        // Start collecting for the new year
        // Note: Snapshots are created when all reports arrive (in handle_syndicate_report),
        // not when the Year event arrives, since syndicates report AFTER receiving the Year event

        // Integer division: year 0 = days 0-364, year 1 = days 365-729, etc.
        self.current_year = current_t / DAYS_PER_YEAR;
        self.current_day = current_t;
        self.pending_reports.clear();

        // Reset catastrophe tracking for new year
        self.cat_event_occurred = false;
        self.cat_event_total_loss = 0.0;
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_syndicate_report(
        &mut self,
        syndicate_id: usize,
        capital: f64,
        annual_premiums: f64,
        annual_claims: f64,
        num_policies: usize,
        num_claims: usize,
        markup_m_t: f64,
        uniform_deviation: f64,
    ) -> Vec<(usize, Event)> {
        // Store the report
        self.pending_reports.insert(
            syndicate_id,
            SyndicateReport {
                capital,
                is_insolvent: capital <= 0.0,
                annual_premiums,
                annual_claims,
                num_policies,
                num_claims,
                markup_m_t,
                uniform_deviation,
            },
        );

        // Check if we have all reports
        if self.pending_reports.len() == self.num_syndicates {
            self.create_snapshot()
        } else {
            Vec::new()
        }
    }

    fn create_snapshot(&mut self) -> Vec<(usize, Event)> {
        // Calculate aggregate statistics
        let total_capital: f64 = self.pending_reports.values().map(|r| r.capital).sum();
        let num_solvent = self
            .pending_reports
            .values()
            .filter(|r| !r.is_insolvent)
            .count();
        let num_insolvent = self.num_syndicates - num_solvent;

        // Calculate premium and loss metrics from syndicate reports
        let total_annual_premiums: f64 = self
            .pending_reports
            .values()
            .map(|r| r.annual_premiums)
            .sum();
        let total_annual_claims: f64 = self.pending_reports.values().map(|r| r.annual_claims).sum();
        let total_annual_policies: usize =
            self.pending_reports.values().map(|r| r.num_policies).sum();
        let total_annual_claims_count: usize =
            self.pending_reports.values().map(|r| r.num_claims).sum();

        // Average premium per policy (market-wide) for this year
        let avg_premium = if total_annual_policies > 0 {
            total_annual_premiums / total_annual_policies as f64
        } else {
            0.0
        };

        // Average loss ratio (market-wide)
        let avg_loss_ratio = if total_annual_premiums > 0.0 {
            total_annual_claims / total_annual_premiums
        } else {
            0.0
        };

        // NEW: Calculate premium standard deviation
        let premium_values: Vec<f64> = self
            .pending_reports
            .values()
            .filter(|r| r.num_policies > 0)
            .map(|r| r.annual_premiums / r.num_policies as f64)
            .collect();
        let premium_std_dev = if premium_values.len() > 1 {
            let mean = premium_values.iter().sum::<f64>() / premium_values.len() as f64;
            let variance = premium_values
                .iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>()
                / premium_values.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };

        // NEW: Calculate markup metrics
        let markup_values: Vec<f64> = self
            .pending_reports
            .values()
            .map(|r| r.markup_m_t)
            .collect();
        let markup_avg = if !markup_values.is_empty() {
            markup_values.iter().sum::<f64>() / markup_values.len() as f64
        } else {
            0.0
        };
        let markup_std_dev = if markup_values.len() > 1 {
            let variance = markup_values
                .iter()
                .map(|v| (v - markup_avg).powi(2))
                .sum::<f64>()
                / markup_values.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };

        // NEW: Calculate average uniform deviation
        let avg_uniform_deviation = if !self.pending_reports.is_empty() {
            self.pending_reports
                .values()
                .map(|r| r.uniform_deviation)
                .sum::<f64>()
                / self.pending_reports.len() as f64
        } else {
            0.0
        };

        // Calculate industry-wide loss statistics from syndicate participations
        //
        // IMPORTANT: These statistics are PERPARTICIPATION, not per-risk:
        // - industry_claim_frequency = claims received / participations
        // - industry_avg_claim_cost = avg claim amount received (line-share adjusted)
        //
        // When syndicates use these for pricing, they should NOT multiply by line_size again,
        // since the statistics already reflect participation-level expectations.

        let industry_claim_frequency = if total_annual_policies > 0 {
            total_annual_claims_count as f64 / total_annual_policies as f64
        } else {
            0.0
        };

        let industry_avg_claim_cost = if total_annual_claims_count > 0 {
            total_annual_claims / total_annual_claims_count as f64
        } else {
            0.0
        };

        // Create market-level snapshot
        let snapshot = MarketSnapshot {
            year: self.current_year,
            day: self.current_day,
            avg_premium,
            avg_loss_ratio,
            num_solvent_syndicates: num_solvent,
            num_insolvent_syndicates: num_insolvent,
            total_capital,
            total_policies: total_annual_policies,
            premium_std_dev,
            markup_avg,
            markup_std_dev,
            cat_event_occurred: self.cat_event_occurred,
            cat_event_loss: self.cat_event_total_loss,
            avg_uniform_deviation,
        };

        self.time_series.snapshots.push(snapshot);

        // NEW: Create per-syndicate snapshots
        for (syndicate_id, report) in &self.pending_reports {
            let loss_ratio = if report.annual_premiums > 0.0 {
                report.annual_claims / report.annual_premiums
            } else {
                0.0
            };

            self.syndicate_time_series
                .snapshots
                .push(crate::SyndicateSnapshot {
                    year: self.current_year,
                    syndicate_id: *syndicate_id,
                    capital: report.capital,
                    markup_m_t: report.markup_m_t,
                    loss_ratio,
                    num_policies: report.num_policies,
                    annual_premiums: report.annual_premiums,
                    annual_claims: report.annual_claims,
                });
        }

        // Clear pending reports after creating snapshot
        self.pending_reports.clear();

        // Emit industry loss statistics and pricing statistics for syndicates to use
        vec![
            (
                self.current_day,
                Event::IndustryLossStatsReported {
                    avg_claim_frequency: industry_claim_frequency,
                    avg_claim_cost: industry_avg_claim_cost,
                },
            ),
            (
                self.current_day,
                Event::IndustryPricingStatsReported { avg_premium },
            ),
        ]
    }
}

impl Agent<Event, Stats> for MarketStatisticsCollector {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::Year => {
                self.handle_year(current_t);
                Response::new()
            }
            Event::SyndicateCapitalReported {
                syndicate_id,
                capital,
                annual_premiums,
                annual_claims,
                num_policies,
                num_claims,
                markup_m_t,
                uniform_deviation,
            } => {
                let events = self.handle_syndicate_report(
                    *syndicate_id,
                    *capital,
                    *annual_premiums,
                    *annual_claims,
                    *num_policies,
                    *num_claims,
                    *markup_m_t,
                    *uniform_deviation,
                );
                Response::events(events)
            }
            Event::YearEndCatastropheReport {
                year: _,
                total_loss,
                num_events,
            } => {
                // Track catastrophe occurrence for current year
                if *num_events > 0 {
                    self.cat_event_occurred = true;
                    self.cat_event_total_loss += total_loss;
                }
                Response::new()
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        // Return combined market stats (both market-level and syndicate-level time series)
        Stats::CombinedMarketStats(CombinedMarketStats {
            market_series: self.time_series.clone(),
            syndicate_series: self.syndicate_time_series.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_initializes() {
        let collector = MarketStatisticsCollector::new(5);
        assert_eq!(collector.num_syndicates, 5);
        assert_eq!(collector.time_series.snapshots.len(), 0);
    }

    #[test]
    fn test_collector_creates_snapshot_when_all_reports_received() {
        let mut collector = MarketStatisticsCollector::new(2);

        // Simulate Year event (using act method)
        collector.act(365, &Event::Year);

        // Receive reports from both syndicates (using act method)
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 0,
                capital: 10_000_000.0,
                annual_premiums: 1_000_000.0,
                annual_claims: 800_000.0,
                num_policies: 10,
                num_claims: 1,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        );
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 1,
                capital: 9_500_000.0,
                annual_premiums: 1_200_000.0,
                annual_claims: 900_000.0,
                num_policies: 12,
                num_claims: 1,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        );

        // Should have created one snapshot
        assert_eq!(collector.time_series.snapshots.len(), 1);
        let snapshot = &collector.time_series.snapshots[0];
        assert_eq!(snapshot.year, 1);
        assert_eq!(snapshot.num_solvent_syndicates, 2);
        assert_eq!(snapshot.num_insolvent_syndicates, 0);
        assert_eq!(snapshot.total_capital, 19_500_000.0);
    }

    #[test]
    fn test_collector_tracks_insolvency() {
        let mut collector = MarketStatisticsCollector::new(3);

        collector.act(365, &Event::Year);
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 0,
                capital: 10_000_000.0,
                annual_premiums: 1_000_000.0,
                annual_claims: 800_000.0,
                num_policies: 10,
                num_claims: 1,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        ); // Solvent
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 1,
                capital: -500_000.0,
                annual_premiums: 500_000.0,
                annual_claims: 1_000_000.0,
                num_policies: 5,
                num_claims: 2,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        ); // Insolvent
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 2,
                capital: 8_000_000.0,
                annual_premiums: 900_000.0,
                annual_claims: 700_000.0,
                num_policies: 9,
                num_claims: 1,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        ); // Solvent

        assert_eq!(collector.time_series.snapshots.len(), 1);
        let snapshot = &collector.time_series.snapshots[0];
        assert_eq!(snapshot.num_solvent_syndicates, 2);
        assert_eq!(snapshot.num_insolvent_syndicates, 1);
    }

    #[test]
    fn test_collector_accumulates_snapshots_over_years() {
        let mut collector = MarketStatisticsCollector::new(2);

        // Year 1
        collector.act(365, &Event::Year);
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 0,
                capital: 10_000_000.0,
                annual_premiums: 1_000_000.0,
                annual_claims: 800_000.0,
                num_policies: 10,
                num_claims: 1,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        );
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 1,
                capital: 9_500_000.0,
                annual_premiums: 1_200_000.0,
                annual_claims: 900_000.0,
                num_policies: 12,
                num_claims: 1,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        );
        // Snapshot 1 created when all year 1 reports received

        // Year 2
        collector.act(730, &Event::Year);
        // Snapshot is finalized on Year event if all reports received (this creates snapshot 1 again if pending)
        collector.act(
            730,
            &Event::SyndicateCapitalReported {
                syndicate_id: 0,
                capital: 11_000_000.0,
                annual_premiums: 1_100_000.0,
                annual_claims: 850_000.0,
                num_policies: 11,
                num_claims: 1,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        );
        collector.act(
            730,
            &Event::SyndicateCapitalReported {
                syndicate_id: 1,
                capital: 9_000_000.0,
                annual_premiums: 1_000_000.0,
                annual_claims: 950_000.0,
                num_policies: 10,
                num_claims: 1,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        );
        // Snapshot 2 created when all year 2 reports received

        // Should have 2 snapshots (one per year completed)
        assert_eq!(
            collector.time_series.snapshots.len(),
            2,
            "Should have one snapshot per year"
        );
        assert_eq!(collector.time_series.snapshots[0].year, 1);
        assert_eq!(collector.time_series.snapshots[1].year, 2);
    }

    #[test]
    fn test_collector_calculates_premium_metrics() {
        let mut collector = MarketStatisticsCollector::new(2);

        collector.act(365, &Event::Year);
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 0,
                capital: 10_000_000.0,
                annual_premiums: 2_000_000.0, // 20 policies × $100k each
                annual_claims: 1_600_000.0,   // Loss ratio = 0.8
                num_policies: 20,
                num_claims: 2,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        );
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 1,
                capital: 9_500_000.0,
                annual_premiums: 3_000_000.0, // 30 policies × $100k each
                annual_claims: 2_400_000.0,   // Loss ratio = 0.8
                num_policies: 30,
                num_claims: 3,
                markup_m_t: 0.0,
                uniform_deviation: 0.0,
            },
        );

        assert_eq!(collector.time_series.snapshots.len(), 1);
        let snapshot = &collector.time_series.snapshots[0];

        // Check premium metrics
        assert_eq!(snapshot.total_policies, 50); // 20 + 30
        assert_eq!(snapshot.avg_premium, 100_000.0); // $5M total / 50 policies
        assert_eq!(snapshot.avg_loss_ratio, 0.8); // $4M claims / $5M premiums
    }
}
