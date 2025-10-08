use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

struct Event<T> {
    t: usize,
    data: T,
}

impl<T> PartialEq for Event<T> {
    fn eq(&self, other: &Self) -> bool {
        self.t == other.t
    }
}

impl<T> Eq for Event<T> {}

impl<T> Ord for Event<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.t.cmp(&self.t)
    }
}

impl<T> PartialOrd for Event<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Response<T, S> {
    pub events: Vec<(usize, T)>,
    pub agents: Vec<Box<dyn Agent<T, S>>>,
}

impl<T, S> Default for Response<T, S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, S> Response<T, S> {
    pub fn new() -> Response<T, S> {
        Response {
            events: Vec::<(usize, T)>::new(),
            agents: Vec::<Box<dyn Agent<T, S>>>::new(),
        }
    }

    pub fn event(t: usize, event: T) -> Response<T, S> {
        Response {
            events: vec![(t, event)],
            agents: Vec::<Box<dyn Agent<T, S>>>::new(),
        }
    }

    pub fn events(events: Vec<(usize, T)>) -> Response<T, S> {
        Response {
            events,
            agents: Vec::<Box<dyn Agent<T, S>>>::new(),
        }
    }
}

pub trait Agent<T, S> {
    fn act(&mut self, _current_t: usize, _data: &T) -> Response<T, S> {
        Response::new()
    }

    fn stats(&self) -> S;
}

pub struct EventLoop<T, S> {
    queue: BinaryHeap<Event<T>>,
    current_t: usize,
    agents: Vec<Box<dyn Agent<T, S>>>,
}

impl<T, S> EventLoop<T, S> {
    pub fn stats(&self) -> Vec<S> {
        let s = self.agents.iter().map(|agent| agent.stats()).collect();
        s
    }
    pub fn new(events: Vec<(usize, T)>, agents: Vec<Box<dyn Agent<T, S>>>) -> EventLoop<T, S> {
        let outer_events: Vec<Event<T>> = events
            .into_iter()
            .map(|(t, data)| Event { t, data })
            .collect();
        EventLoop {
            agents,
            queue: BinaryHeap::from(outer_events),
            current_t: 0,
        }
    }

    fn broadcast(&mut self) {
        if let Some(event) = self.queue.pop() {
            self.current_t = event.t;
            let mut new_agents = Vec::<Box<dyn Agent<T, S>>>::new();
            for agent in &mut self.agents {
                let response = agent.act(self.current_t, &event.data);
                for new_event in response.events {
                    if new_event.0 >= self.current_t {
                        self.queue.push(Event {
                            t: new_event.0,
                            data: new_event.1,
                        })
                    }
                }
                for new_agent in response.agents {
                    new_agents.push(new_agent);
                }
            }
            for new_agent in new_agents {
                self.agents.push(new_agent);
            }
        }
    }

    pub fn run(&mut self, until: usize) {
        while self.queue.peek().is_some() {
            if self.current_t >= until {
                return;
            }
            self.broadcast();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let event = Event::<u8> { t: 1, data: 1 };
        assert_eq!(event.t, 1);
    }

    #[test]
    fn min_queue() {
        let mut queue = BinaryHeap::<Event<u8>>::new();
        queue.push(Event::<u8> { t: 2, data: 2 });
        queue.push(Event::<u8> { t: 1, data: 1 });
        if let Some(first) = queue.peek() {
            assert_eq!(first.data, 1);
        }
    }

    #[test]
    fn noddy_run() {
        struct NoddyAgent {}
        enum Stats {
            A,
        }
        impl Agent<u8, Stats> for NoddyAgent {
            fn stats(&self) -> Stats {
                Stats::A
            }
        }

        let queue: BinaryHeap<Event<u8>> =
            BinaryHeap::from([Event { t: 1, data: 1 }, Event { t: 2, data: 2 }]);
        let agents: Vec<Box<dyn Agent<u8, Stats>>> = vec![Box::new(NoddyAgent {})];

        let mut event_loop = EventLoop {
            queue,
            current_t: 0,
            agents,
        };

        event_loop.run(10);

        assert_eq!(event_loop.current_t, 2)
    }

    #[test]
    fn new_event() {
        struct NoddyAgent {}
        enum Stats {
            A,
        }
        impl Agent<u8, Stats> for NoddyAgent {
            fn act(&mut self, current_t: usize, _data: &u8) -> Response<u8, Stats> {
                Response::event(current_t + 1, 0)
            }
            fn stats(&self) -> Stats {
                Stats::A
            }
        }
        let mut event_loop = EventLoop::new(vec![(0, 1)], vec![Box::new(NoddyAgent {})]);
        event_loop.run(10);
        assert_eq!(event_loop.current_t, 10);
    }

    #[test]
    fn new_agent() {
        struct NoddyAgent {}
        enum Stats {
            A,
        }
        impl Agent<u8, Stats> for NoddyAgent {
            fn act(&mut self, _current_t: usize, _data: &u8) -> Response<u8, Stats> {
                Response {
                    events: Vec::<(usize, u8)>::new(),
                    agents: vec![Box::new(NoddyAgent {})],
                }
            }
            fn stats(&self) -> Stats {
                Stats::A
            }
        }
        let queue: BinaryHeap<Event<u8>> =
            BinaryHeap::from([Event { t: 1, data: 1 }, Event { t: 2, data: 2 }]);
        let agents: Vec<Box<dyn Agent<u8, Stats>>> = vec![Box::new(NoddyAgent {})];

        let mut event_loop = EventLoop {
            queue,
            current_t: 0,
            agents,
        };

        event_loop.run(10);

        // First event: 1 new agent
        // Second event: 2 new agents
        assert_eq!(event_loop.agents.len(), 4)
    }
}
