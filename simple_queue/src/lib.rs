use std::collections::{HashSet, VecDeque};

use rand_distr::{Distribution, Geometric, Normal};

#[derive(Debug, Clone)]
pub struct ConsumerStats {}

impl Default for ConsumerStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsumerStats {
    pub fn new() -> Self {
        ConsumerStats {}
    }
}

#[derive(Debug, Clone)]
pub struct ResourceStats {
    arrival_count: usize,
    acquired_count: usize,
    expiry_count: usize,
    consume_sum: usize,
    wait_sum: usize,
}

impl Default for ResourceStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceStats {
    pub fn new() -> Self {
        ResourceStats {
            arrival_count: 0,
            acquired_count: 0,
            consume_sum: 0,
            expiry_count: 0,
            wait_sum: 0,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Start,
    ResourceRequested(usize, usize), // resource ID, consumer ID
    ResourceRequestExpired(usize, usize, usize), // resource ID, consumer ID, requested t
    ResourceReleased(usize, usize, usize), // resource ID, consumer ID, requested t
    ResourceAcquired(usize, usize, usize), // resource ID, consumer ID, requested t
}

#[derive(Debug)]
pub enum Stats {
    ConsumerStats(ConsumerStats),
    ResourceStats(ResourceStats),
}

#[derive(Debug)]
pub struct ConsumerProcess {
    resource_id: usize,
    next_consumer_id: usize,
    arrival_interval: Geometric,
    consume_duration: Normal<f64>,
    wait_duration: Normal<f64>,
    stats: ConsumerStats,
}

impl ConsumerProcess {
    pub fn new(
        resource_id: usize,
        arrival_interval: f64,
        consume_duration: (f64, f64),
        wait_duration: (f64, f64),
    ) -> ConsumerProcess {
        ConsumerProcess {
            resource_id,
            next_consumer_id: 0,
            arrival_interval: Geometric::new(arrival_interval).unwrap(),
            consume_duration: Normal::new(consume_duration.0, consume_duration.1).unwrap(),
            wait_duration: Normal::new(wait_duration.0, wait_duration.1).unwrap(),
            stats: ConsumerStats::new(),
        }
    }

    fn draw_arrival_interval(&self) -> usize {
        self.arrival_interval.sample(&mut rand::rng()) as usize
    }

    fn draw_consume_duration(&self) -> usize {
        self.consume_duration
            .sample(&mut rand::rng())
            .floor()
            .max(0.0) as usize
    }

    fn draw_wait_duration(&self) -> usize {
        self.wait_duration.sample(&mut rand::rng()).floor().max(0.0) as usize
    }

    fn new_consumer(&mut self, current_t: usize) -> ((usize, Event), (usize, Event)) {
        let consumer_id = self.next_consumer_id;
        self.next_consumer_id += 1;
        let arrival_interval = self.draw_arrival_interval();
        let wait_duration = self.draw_wait_duration();
        let request = (
            current_t + arrival_interval,
            Event::ResourceRequested(self.resource_id, consumer_id),
        );
        let expire = (
            current_t + arrival_interval + wait_duration,
            Event::ResourceRequestExpired(
                self.resource_id,
                consumer_id,
                current_t + arrival_interval,
            ),
        );
        (request, expire)
    }
}

impl des::Agent<Event, Stats> for ConsumerProcess {
    fn stats(&self) -> Stats {
        Stats::ConsumerStats(self.stats.clone())
    }
    fn act(&mut self, current_t: usize, data: &Event) -> des::Response<Event, Stats> {
        // println!("[{}] ConsumerProcess {:#?}", current_t, data);
        match data {
            Event::Start => {
                let (request, expire) = self.new_consumer(current_t);
                let events = vec![request, expire];
                des::Response::events(events)
            }
            Event::ResourceAcquired(rid, cid, _requested_t) => {
                if &self.resource_id != rid {
                    return des::Response::new();
                }
                println!("[{}] Consumer {} acquired Resource {}", current_t, cid, rid);
                let consume_duration = self.draw_consume_duration();
                let release = (
                    current_t + consume_duration,
                    Event::ResourceReleased(self.resource_id, *cid, current_t),
                );
                des::Response::event(release.0, release.1)
            }
            Event::ResourceRequested(rid, _cid) => {
                if &self.resource_id != rid {
                    return des::Response::new();
                }

                let (request, expire) = self.new_consumer(current_t);
                let events = vec![request, expire];
                des::Response::events(events)
            }
            _ => des::Response::new(),
        }
    }
}

pub struct Resource {
    resource_id: usize,
    consumer_total: usize,
    consumer_count: usize,
    consumer_queue: VecDeque<(usize, usize)>, // consumer_id, requested_t
    consumers_active: HashSet<usize>,
    stats: ResourceStats,
}

impl Resource {
    pub fn new(resource_id: usize, consumer_total: usize) -> Resource {
        Resource {
            resource_id,
            consumer_total,
            consumer_count: 0,
            consumer_queue: VecDeque::new(),
            consumers_active: HashSet::new(),
            stats: ResourceStats::new(),
        }
    }
}

impl des::Agent<Event, Stats> for Resource {
    fn stats(&self) -> Stats {
        Stats::ResourceStats(self.stats.clone())
    }
    fn act(&mut self, current_t: usize, data: &Event) -> des::Response<Event, Stats> {
        // println!("[{}] Resource {:#?}", current_t, data);
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
                self.stats.arrival_count += 1;
                if self.consumer_total == self.consumer_count {
                    // resource occupied and we add the consumer to the queue
                    // the consumer is active until / unless the resource request expires
                    // println!("  Resource {} fully occupied", self.resource_id);
                    self.consumer_queue.push_back((*cid, current_t));
                    self.consumers_active.insert(*cid);
                    // there's nothing really to do here
                    des::Response::new()
                } else {
                    // resource not fully occupied
                    // increment the count of consumers
                    // if the consumer has acquired the resource at the point of asking
                    // then there is no need to modify the queue or active user set
                    // println!("  Resource {} not fully occupied", self.resource_id);
                    self.consumer_count += 1;
                    self.stats.acquired_count += 1;

                    // broadcast that the consumer has acquired the resource
                    des::Response::event(current_t, Event::ResourceAcquired(*rid, *cid, current_t))
                }
            }
            Event::ResourceReleased(rid, cid, acquired_t) => {
                // skip if the event has nothing to do with this resource
                if rid != &self.resource_id {
                    return des::Response::new();
                }
                println!("[{}] Consumer {} released Resource {}", current_t, cid, rid);
                self.stats.consume_sum += current_t - acquired_t;

                self.consumer_count -= 1;

                // println!("  Checking for queued consumers ...");
                while let Some((consumer_id, requested_t)) = self.consumer_queue.pop_front() {
                    // println!("    {}", consumer_id);
                    if self.consumers_active.contains(&consumer_id) {
                        self.consumers_active.remove(&consumer_id);
                        self.consumer_count += 1;
                        self.stats.wait_sum += current_t - requested_t;
                        self.stats.acquired_count += 1;
                        return des::Response::event(
                            current_t,
                            Event::ResourceAcquired(*rid, consumer_id, requested_t),
                        );
                    } else {
                        // println!(
                        //     "    Consumer {} request for Resource {} had expired",
                        //     consumer_id,
                        //     rid
                        // );
                    };
                }
                // println!("  No consumers waiting for Resource {}", rid);
                des::Response::new()
            }
            Event::ResourceRequestExpired(rid, cid, requested_t) => {
                // skip if the event has nothing to do with this resource
                if rid != &self.resource_id {
                    return des::Response::new();
                }

                let removed = self.consumers_active.remove(cid);

                if removed {
                    println!(
                        "[{}] Consumer {} request for Resource {} expired",
                        current_t, cid, rid
                    );
                    self.stats.expiry_count += 1;
                    self.stats.wait_sum += current_t - requested_t;
                }
                des::Response::new()
            }
            _ => des::Response::new(),
        }
    }
}
