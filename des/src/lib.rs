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

pub trait Agent<T> {
    fn act(&self, current_t: usize, data: &T) -> Vec<(usize, T)> {
        Vec::<(usize, T)>::new()
    }
}

struct EventLoop<T> {
    queue: BinaryHeap<Event<T>>,
    current_t: usize,
    agents: Vec<Box<dyn Agent<T>>>,
}

impl<T> EventLoop<T> {
    fn broadcast(&mut self) {
        if let Some(event) = self.queue.pop() {
            self.current_t = event.t;
            for agent in &mut self.agents {
                let new_events = agent.act(self.current_t, &event.data);
                for new_event in new_events {
                    if new_event.0 <= self.current_t {
                        self.queue.push(Event {
                            t: new_event.0,
                            data: new_event.1,
                        })
                    }
                }
            }
        }
    }

    pub fn run(&mut self) {
        self.broadcast();
        if self.queue.len() > 0 {
            return;
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

        let event_loop = EventLoop {
            queue,
            current_t: 0,
            agents,
        };
    }
}
