use crate::{Event, Stats};
use des::{Agent, Response};

/// Time generator that emits Day, Month, and Year events
#[derive(Default)]
pub struct TimeGenerator {
    current_day: usize,
}

impl TimeGenerator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Agent<Event, Stats> for TimeGenerator {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::Day => {
                self.current_day += 1;
                let mut events = vec![(current_t + 1, Event::Day)];

                // Emit Month event every 30 days
                if self.current_day.is_multiple_of(30) {
                    events.push((current_t, Event::Month));
                }

                // Emit Year event every 365 days
                if self.current_day.is_multiple_of(365) {
                    events.push((current_t, Event::Year));
                }

                Response::events(events)
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        // TimeGenerator doesn't produce stats
        Stats::BrokerStats(crate::BrokerStats::new(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_generator_daily() {
        let mut time_gen = TimeGenerator::new();
        let resp = time_gen.act(0, &Event::Day);
        assert_eq!(resp.events.len(), 1);
        assert!(matches!(resp.events[0].1, Event::Day));
    }

    #[test]
    fn test_time_generator_monthly() {
        let mut time_gen = TimeGenerator::new();
        // Advance 29 days
        for _ in 0..29 {
            time_gen.act(0, &Event::Day);
        }
        // 30th day should emit Month
        let resp = time_gen.act(30, &Event::Day);
        assert!(resp.events.iter().any(|(_, e)| matches!(e, Event::Month)));
    }

    #[test]
    fn test_time_generator_yearly() {
        let mut time_gen = TimeGenerator::new();
        // Advance 364 days
        for _ in 0..364 {
            time_gen.act(0, &Event::Day);
        }
        // 365th day should emit Year
        let resp = time_gen.act(365, &Event::Day);
        assert!(resp.events.iter().any(|(_, e)| matches!(e, Event::Year)));
    }
}
