use std::collections::{HashSet, VecDeque};

enum Event {
    ResourceRequested(usize, usize), // resource ID, consumer ID
    ResourceReleased(usize, usize),  // resource ID, consumer ID
    ResourceAcquired(usize, usize),  // resource ID, consumer ID
}

struct Consumer {
    consumer_id: usize,
}

impl des::Agent<Event> for Consumer {
    fn act(&mut self, current_t: usize, data: &Event) -> des::Response<Event> {
        match data {
            Event::ResourceAcquired(rid, cid) => {
                if cid != &self.consumer_id {
                    return des::Response::new();
                }
                println!("[{}] Consumer {} acquired Resource {}", current_t, cid, rid);
                des::Response::new()
            }
            _ => des::Response::new(),
        }
    }
}

struct Resource {
    resource_id: usize,
    consumer_total: usize,
    consumer_count: usize,
    consumer_queue: VecDeque<usize>,
    consumers_active: HashSet<usize>,
}

impl Resource {
    fn new(resource_id: usize, consumer_total: usize) -> Resource {
        Resource {
            resource_id,
            consumer_total,
            consumer_count: 0,
            consumer_queue: VecDeque::new(),
            consumers_active: HashSet::new(),
        }
    }
}

impl des::Agent<Event> for Resource {
    fn act(&mut self, current_t: usize, data: &Event) -> des::Response<Event> {
        match data {
            Event::ResourceRequested(rid, cid) => {
                // skip if the event has nothing to do with this resource
                if rid != &self.resource_id {
                    return des::Response::new();
                }

                // consumer has attempted to acquire the resource
                println!(
                    "[{}] Consumer {} requested Resource {}",
                    current_t, cid, rid
                );
                if self.consumer_total == self.consumer_count {
                    // resource occupied and we add the consumer to the queue
                    // the consumer is active until / unless the resource request expires
                    println!("  Resource {} fully occupied", self.resource_id);
                    self.consumer_queue.push_back(*cid);
                    self.consumers_active.insert(*cid);
                    // there's nothing really to do here
                    // the consumer has already added a MaybeResourceRequestExpired event
                    des::Response::new()
                } else {
                    // resource not fully occupied
                    // increment the count of consumers
                    // if the consumer has acquired the resource at the point of asking
                    // then there is no need to modify the queue or active user set
                    println!("  Resource {} not fully occupied", self.resource_id);
                    self.consumer_count += 1;

                    // broadcast that the consumer has acquired the resource
                    des::Response::event(current_t, Event::ResourceAcquired(*rid, *cid))
                }
            }
            Event::ResourceReleased(rid, cid) => {
                // skip if the event has nothing to do with this resource
                if rid != &self.resource_id {
                    return des::Response::new();
                }
                println!("[{}] Consumer {} released Resource {}", current_t, cid, rid);

                self.consumer_count -= 1;

                while let Some(consumer_id) = self.consumer_queue.pop_front() {
                    if self.consumers_active.contains(&consumer_id) {
                        self.consumers_active.remove(&consumer_id);
                        self.consumer_count += 1;
                        return des::Response::event(
                            current_t,
                            Event::ResourceAcquired(consumer_id, *rid),
                        );
                    } else {
                        println!(
                            "  Consumer {} request for Resource {} had expired",
                            consumer_id, rid
                        );
                    };
                }
                println!("  No consumers waiting for Resource {}", rid);
                des::Response::new()
            }
            _ => des::Response::new(),
        }
    }
}

fn main() {
    println!("DES: Simple Queue");

    let events = vec![
        (1, Event::ResourceRequested(0, 0)),
        (5, Event::ResourceReleased(0, 0)),
    ];
    let agents: Vec<Box<dyn des::Agent<Event>>> = vec![
        Box::new(Consumer { consumer_id: 0 }),
        Box::new(Resource::new(0, 1)),
    ];

    let mut event_loop = des::EventLoop::new(events, agents);

    event_loop.run()
}
