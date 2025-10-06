enum Event {
    ResourceRequested,
}

struct Consumer {}

impl<Event> des::Agent<Event> for Consumer {
    fn act(&self, _current_t: usize, _data: &Event) -> des::Response<Event> {
        des::Response::new()
    }
}

fn main() {
    println!("DES: Simple Queue");

    let events = vec![(1, Event::ResourceRequested)];
    let agents: Vec<Box<dyn des::Agent<Event>>> = vec![Box::new(Consumer {})];

    let mut event_loop = des::EventLoop::new(events, agents);

    event_loop.run()
}
