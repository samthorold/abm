use crate::{Event, MarketSnapshot, Stats, TimeSeriesStats};
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

    // Temporary storage for collecting stats within a single time period
    current_year: usize,
    current_day: usize,
    pending_reports: HashMap<usize, SyndicateReport>,
}

#[derive(Debug, Clone)]
struct SyndicateReport {
    capital: f64,
    is_insolvent: bool,
    annual_premiums: f64,
    annual_claims: f64,
    num_policies: usize, // Annual policies written (not cumulative)
    num_claims: usize,   // Annual claims received (not cumulative)
}

impl MarketStatisticsCollector {
    pub fn new(num_syndicates: usize) -> Self {
        Self {
            num_syndicates,
            time_series: TimeSeriesStats::new(),
            current_year: 0,
            current_day: 0,
            pending_reports: HashMap::new(),
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
    }

    fn handle_syndicate_report(
        &mut self,
        syndicate_id: usize,
        capital: f64,
        annual_premiums: f64,
        annual_claims: f64,
        num_policies: usize,
        num_claims: usize,
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

        let snapshot = MarketSnapshot {
            year: self.current_year,
            day: self.current_day,
            avg_premium,
            avg_loss_ratio,
            num_solvent_syndicates: num_solvent,
            num_insolvent_syndicates: num_insolvent,
            total_capital,
            total_policies: total_annual_policies,
        };

        self.time_series.snapshots.push(snapshot);

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
            } => {
                let events = self.handle_syndicate_report(
                    *syndicate_id,
                    *capital,
                    *annual_premiums,
                    *annual_claims,
                    *num_policies,
                    *num_claims,
                );
                Response::events(events)
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        // Return time series stats (market-level aggregated data across all syndicates)
        Stats::TimeSeriesStats(self.time_series.clone())
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
