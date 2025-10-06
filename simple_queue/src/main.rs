enum Event {
    ResourceRequested(usize, usize), // resource ID, consumer ID
    ResourceReleased(usize, usize),  // resource ID, consumer ID
    ResourceAcquired(usize, usize),  // resource ID, consumer ID
}

struct Consumer {
    consumer_id: usize,
}

impl des::Agent<Event> for Consumer {
    fn act(&self, _current_t: usize, _data: &Event) -> des::Response<Event> {
        des::Response::new()
    }
}

struct Resource {
    resource_id: usize,
}

impl des::Agent<Event> for Resource {
    fn act(&self, _current_t: usize, data: &Event) -> des::Response<Event> {
        match data {
            Event::ResourceRequested(rid, cid) => {
                println!("Consumer {} requested Resource {}", cid, rid);
                des::Response::new()
            }
            _ => des::Response::new(),
        }
    }
}

fn main() {
    println!("DES: Simple Queue");

    let events = vec![(1, Event::ResourceRequested(0, 0))];
    let agents: Vec<Box<dyn des::Agent<Event>>> = vec![
        Box::new(Consumer { consumer_id: 0 }),
        Box::new(Resource { resource_id: 0 }),
    ];

    let mut event_loop = des::EventLoop::new(events, agents);

    event_loop.run()
}
