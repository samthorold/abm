// Level 2: Integration tests for multi-agent interactions
// Tests the interaction between ConsumerProcess and Resource agents

use simple_queue::{ConsumerProcess, Event, Resource, Stats};

#[test]
fn given_deterministic_consumer_process_when_simulation_runs_then_predictable_events() {
    // Note: This test requires ConsumerProcess to accept a seeded RNG
    // Current implementation uses rand::rng() directly

    // GIVEN: Deterministic consumer process with known parameters
    // - Arrival interval: deterministic 50 time units (using seed)
    // - Service duration: deterministic 30 time units
    // - Wait duration: deterministic 10 time units
    let mut consumer_process = ConsumerProcess::new_with_seed(
        /*resource_id:*/ 0,
        /*seed:*/ 42,
        /*arrival_interval:*/ 1.0 / 50.0,
        /*consume_duration:*/ (30.0, 0.1), // low variance for predictability
        /*wait_duration:*/ (10.0, 0.1),
    );

    // WHEN: Start event triggers first consumer
    let response = consumer_process.act(0, &Event::Start);

    // THEN: Two events scheduled (request + expiry)
    assert_eq!(
        response.events.len(),
        2,
        "Should schedule request and expiry"
    );

    // Extract request and expiry times
    let mut request_event = None;
    let mut expiry_event = None;

    for (t, event) in &response.events {
        match event {
            Event::ResourceRequested(rid, cid) => {
                assert_eq!(*rid, 0);
                assert_eq!(*cid, 0);
                request_event = Some(*t);
            }
            Event::ResourceRequestExpired(rid, cid, req_t) => {
                assert_eq!(*rid, 0);
                assert_eq!(*cid, 0);
                expiry_event = Some((*t, *req_t));
            }
            _ => panic!("Unexpected event type"),
        }
    }

    // THEN: Request scheduled in future
    let request_t = request_event.expect("Should have request event");
    assert!(request_t > 0, "Request should be scheduled in future");

    // THEN: Expiry scheduled after request
    let (expiry_t, expiry_req_t) = expiry_event.expect("Should have expiry event");
    assert_eq!(
        expiry_req_t, request_t,
        "Expiry should reference request time"
    );
    assert!(expiry_t > request_t, "Expiry should be after request");

    // Expected: ~10 time units between request and expiry
    let wait_duration = expiry_t - request_t;
    assert!(
        wait_duration >= 5 && wait_duration <= 15,
        "Wait duration should be ~10 units, got {}",
        wait_duration
    );
}

#[test]
fn given_consumer_and_resource_when_acquisition_occurs_then_release_scheduled() {
    // GIVEN: Consumer process and resource
    let mut consumer_process =
        ConsumerProcess::new_with_seed(0, 42, 1.0 / 50.0, (30.0, 1.0), (10.0, 1.0));
    let mut resource = Resource::new(0, 1);

    // WHEN: Consumer requests resource (at t=10)
    let acquire_response = resource.act(10, &Event::ResourceRequested(0, 1));

    // Resource grants acquisition immediately
    assert_eq!(acquire_response.events.len(), 1);
    let (acq_t, acq_event) = &acquire_response.events[0];
    assert_eq!(*acq_t, 10);

    // WHEN: Consumer process handles acquisition event
    let release_response = consumer_process.act(10, acq_event);

    // THEN: Consumer schedules release event
    assert_eq!(release_response.events.len(), 1);

    match &release_response.events[0] {
        (release_t, Event::ResourceReleased(rid, cid, acq_time)) => {
            assert_eq!(*rid, 0);
            assert_eq!(*cid, 1);
            assert_eq!(*acq_time, 10);
            assert!(*release_t > 10, "Release should be scheduled in future");

            // Expected: release ~30 time units after acquisition (service time)
            let service_duration = *release_t - 10;
            assert!(
                service_duration >= 25 && service_duration <= 35,
                "Service duration should be ~30 units, got {}",
                service_duration
            );
        }
        _ => panic!("Expected ResourceReleased event"),
    }
}

#[test]
fn given_consumer_and_resource_when_request_triggers_new_consumer() {
    // GIVEN: Consumer process
    let mut consumer_process =
        ConsumerProcess::new_with_seed(0, 42, 1.0 / 50.0, (30.0, 1.0), (10.0, 1.0));

    // WHEN: First consumer makes a request (triggering arrival of next consumer)
    let response = consumer_process.act(50, &Event::ResourceRequested(0, 0));

    // THEN: New consumer scheduled
    assert_eq!(
        response.events.len(),
        2,
        "Should schedule next consumer request + expiry"
    );

    // Verify next consumer has incremented ID
    for (_, event) in &response.events {
        match event {
            Event::ResourceRequested(_, cid) => {
                assert_eq!(*cid, 1, "Next consumer should have ID 1");
            }
            Event::ResourceRequestExpired(_, cid, _) => {
                assert_eq!(*cid, 1, "Next consumer expiry should have ID 1");
            }
            _ => {}
        }
    }
}

#[test]
fn scenario_full_consumer_lifecycle() {
    // This test simulates a complete lifecycle without using EventLoop
    // Demonstrates the agent interaction protocol

    // GIVEN: One consumer process and one resource
    let mut consumer_process =
        ConsumerProcess::new_with_seed(0, 123, 1.0 / 100.0, (50.0, 5.0), (20.0, 2.0));
    let mut resource = Resource::new(0, 1);

    let mut time = 0usize;
    let mut events: Vec<(usize, Event)> = Vec::new();

    // WHEN: Simulation starts
    let start_response = consumer_process.act(time, &Event::Start);
    events.extend(start_response.events);

    // Extract first request
    events.sort_by_key(|(t, _)| *t);
    let (request_time, request_event) = events.remove(0);
    time = request_time;

    // Consumer requests resource
    let acquire_response = resource.act(time, &request_event);

    // THEN: Resource grants access
    assert_eq!(acquire_response.events.len(), 1);
    events.extend(acquire_response.events);

    // Consumer receives acquisition notification
    events.sort_by_key(|(t, _)| *t);
    let (acq_time, acq_event) = events.remove(0);
    time = acq_time;

    let release_response = consumer_process.act(time, &acq_event);

    // THEN: Consumer schedules release
    assert_eq!(release_response.events.len(), 1);
    events.extend(release_response.events);

    // Process release event
    events.sort_by_key(|(t, _)| *t);
    let (release_time, release_event) = events.remove(0);
    time = release_time;

    let release_response = resource.act(time, &release_event);

    // THEN: Resource successfully released
    // Resource count should be back to 0
    assert_eq!(resource.consumer_count, 0);

    // THEN: Statistics show successful transaction
    assert_eq!(resource.stats.arrival_count, 1);
    assert_eq!(resource.stats.acquired_count, 1);
    assert_eq!(resource.stats.expiry_count, 0);
    assert!(resource.stats.consume_sum > 0);
}

#[test]
fn scenario_resource_contention_and_queueing() {
    // Tests behavior when multiple consumers compete for limited resource

    // GIVEN: Two consumer processes with different seeds, one resource (capacity 1)
    let mut cp1 = ConsumerProcess::new_with_seed(0, 111, 1.0 / 100.0, (50.0, 5.0), (20.0, 2.0));
    let mut cp2 = ConsumerProcess::new_with_seed(0, 222, 1.0 / 100.0, (50.0, 5.0), (20.0, 2.0));
    let mut resource = Resource::new(0, 1);

    // WHEN: Both start simultaneously
    let r1 = cp1.act(0, &Event::Start);
    let r2 = cp2.act(0, &Event::Start);

    // Collect all initial requests
    let mut all_events: Vec<(usize, Event)> = Vec::new();
    all_events.extend(r1.events);
    all_events.extend(r2.events);

    // Find first two requests (filter out expiry events)
    let requests: Vec<_> = all_events
        .iter()
        .filter(|(_, e)| matches!(e, Event::ResourceRequested(_, _)))
        .collect();

    assert_eq!(requests.len(), 2, "Should have two consumer requests");

    // WHEN: First consumer requests (assume at t=10)
    resource.act(10, &requests[0].1);

    // THEN: Resource occupied
    assert_eq!(resource.consumer_count, 1);

    // WHEN: Second consumer requests (assume at t=12)
    let queue_response = resource.act(12, &requests[1].1);

    // THEN: Second consumer queued (no immediate acquisition)
    assert_eq!(
        queue_response.events.len(),
        0,
        "Should be queued, not acquired"
    );
    assert_eq!(resource.consumer_queue.len(), 1, "One consumer in queue");
    assert_eq!(resource.consumer_count, 1, "Still only one active");
}

#[cfg(test)]
mod test_helpers {
    use super::*;

    // Helper to extract specific event types from response
    pub fn extract_events_of_type<F>(
        response: &des::Response<Event, Stats>,
        predicate: F,
    ) -> Vec<(usize, Event)>
    where
        F: Fn(&Event) -> bool,
        Event: Clone,
    {
        response
            .events
            .iter()
            .filter(|(_, e)| predicate(e))
            .cloned()
            .collect()
    }

    // Helper to find next event of specific type in event list
    pub fn next_event_matching<F>(events: &[(usize, Event)], predicate: F) -> Option<(usize, Event)>
    where
        F: Fn(&Event) -> bool,
        Event: Clone,
    {
        events.iter().find(|(_, e)| predicate(e)).cloned()
    }
}
