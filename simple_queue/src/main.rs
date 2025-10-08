fn main() {
    println!("DES: Simple Queue");

    // Bank renege
    // A counter with a random service time and customers who renege.
    //

    let events = vec![(0, simple_queue::Event::Start)];

    let counter_id = 0;
    let concurrent_customers = 1;

    let agents: Vec<Box<dyn des::Agent<simple_queue::Event, simple_queue::Stats>>> = vec![
        Box::new(simple_queue::ConsumerProcess::new(
            counter_id,
            1.0 / 100.0,
            (120.0, 20.0),
            (20.0, 2.0),
        )),
        Box::new(simple_queue::Resource::new(
            counter_id,
            concurrent_customers,
        )),
    ];

    let mut event_loop = des::EventLoop::new(events, agents);

    event_loop.run(1000);

    println!("{:#?}", event_loop.stats());
}
