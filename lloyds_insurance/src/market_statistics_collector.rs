use crate::{Event, MarketSnapshot, Stats, TimeSeriesStats};
use des::{Agent, Response};
use std::collections::HashMap;

/// Collects market statistics over time to build a time series
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
    num_policies: usize,
    total_premiums: f64,
    loss_ratio: f64,
    is_insolvent: bool,
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
        // Create snapshot from previous year's data if we have pending reports
        // (This handles the case where the previous year's snapshot wasn't created yet)
        if self.pending_reports.len() == self.num_syndicates {
            self.create_snapshot();
        }

        // Now start collecting for the new year
        self.current_year = current_t / 365;
        self.current_day = current_t;
        self.pending_reports.clear();
    }

    fn handle_syndicate_report(&mut self, syndicate_id: usize, capital: f64) {
        // Store the report
        self.pending_reports.insert(
            syndicate_id,
            SyndicateReport {
                capital,
                num_policies: 0, // Will need to get from events
                total_premiums: 0.0,
                loss_ratio: 0.0,
                is_insolvent: capital <= 0.0,
            },
        );

        // Check if we have all reports
        if self.pending_reports.len() == self.num_syndicates {
            self.create_snapshot();
        }
    }

    fn create_snapshot(&mut self) {
        // Calculate aggregate statistics
        let total_capital: f64 = self.pending_reports.values().map(|r| r.capital).sum();
        let num_solvent = self
            .pending_reports
            .values()
            .filter(|r| !r.is_insolvent)
            .count();
        let num_insolvent = self.num_syndicates - num_solvent;

        let avg_premium = if num_solvent > 0 {
            self.pending_reports
                .values()
                .filter(|r| !r.is_insolvent)
                .map(|r| r.total_premiums)
                .sum::<f64>()
                / num_solvent as f64
        } else {
            0.0
        };

        let avg_loss_ratio = if num_solvent > 0 {
            self.pending_reports
                .values()
                .filter(|r| !r.is_insolvent)
                .map(|r| r.loss_ratio)
                .sum::<f64>()
                / num_solvent as f64
        } else {
            0.0
        };

        let total_policies: usize = self.pending_reports.values().map(|r| r.num_policies).sum();

        let snapshot = MarketSnapshot {
            year: self.current_year,
            day: self.current_day,
            avg_premium,
            avg_loss_ratio,
            num_solvent_syndicates: num_solvent,
            num_insolvent_syndicates: num_insolvent,
            total_capital,
            total_policies,
        };

        self.time_series.snapshots.push(snapshot);

        // Clear pending reports after creating snapshot
        self.pending_reports.clear();
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
            } => {
                self.handle_syndicate_report(*syndicate_id, *capital);
                Response::new()
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
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
            },
        );
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 1,
                capital: 9_500_000.0,
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
            },
        ); // Solvent
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 1,
                capital: -500_000.0,
            },
        ); // Insolvent
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 2,
                capital: 8_000_000.0,
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
            },
        );
        collector.act(
            365,
            &Event::SyndicateCapitalReported {
                syndicate_id: 1,
                capital: 9_500_000.0,
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
            },
        );
        collector.act(
            730,
            &Event::SyndicateCapitalReported {
                syndicate_id: 1,
                capital: 9_000_000.0,
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
}
