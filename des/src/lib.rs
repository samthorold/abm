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

pub struct Response<T> {
    events: Vec<(usize, T)>,
    agents: Vec<Box<dyn Agent<T>>>,
}

impl<T> Response<T> {
    pub fn new() -> Response<T> {
        Response {
            events: Vec::<(usize, T)>::new(),
            agents: Vec::<Box<dyn Agent<T>>>::new(),
        }
    }
}

pub trait Agent<T> {
    fn act(&self, _current_t: usize, _data: &T) -> Response<T> {
        Response::new()
    }
}

pub struct EventLoop<T> {
    queue: BinaryHeap<Event<T>>,
    current_t: usize,
    agents: Vec<Box<dyn Agent<T>>>,
}

impl<T> EventLoop<T> {
    pub fn new(events: Vec<(usize, T)>, agents: Vec<Box<dyn Agent<T>>>) -> EventLoop<T> {
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
            let mut new_agents = Vec::<Box<dyn Agent<T>>>::new();
            for agent in &mut self.agents {
                let response = agent.act(self.current_t, &event.data);
                for new_event in response.events {
                    if new_event.0 <= self.current_t {
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

    pub fn run(&mut self) {
        while let Some(_) = self.queue.peek() {
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
        impl Agent<u8> for NoddyAgent {}

        let queue: BinaryHeap<Event<u8>> =
            BinaryHeap::from([Event { t: 1, data: 1 }, Event { t: 2, data: 2 }]);
        let agents: Vec<Box<dyn Agent<u8>>> = vec![Box::new(NoddyAgent {})];

        let mut event_loop = EventLoop {
            queue,
            current_t: 0,
            agents,
        };

        event_loop.run();

        assert_eq!(event_loop.current_t, 2)
    }

    #[test]
    fn new_agent() {
        struct NoddyAgent {}
        impl Agent<u8> for NoddyAgent {
            fn act(&self, _current_t: usize, _data: &u8) -> Response<u8> {
                Response {
                    events: Vec::<(usize, u8)>::new(),
                    agents: vec![Box::new(NoddyAgent {})],
                }
            }
        }
        let queue: BinaryHeap<Event<u8>> =
            BinaryHeap::from([Event { t: 1, data: 1 }, Event { t: 2, data: 2 }]);
        let agents: Vec<Box<dyn Agent<u8>>> = vec![Box::new(NoddyAgent {})];

        let mut event_loop = EventLoop {
            queue,
            current_t: 0,
            agents,
        };

        event_loop.run();

        // First event: 1 new agent
        // Second event: 2 new agents
        assert_eq!(event_loop.agents.len(), 4)
    }
}
