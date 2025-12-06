// Demonstration of Given-When-Then testing for agent behavior
// These tests verify Resource agent state transitions

use simple_queue::{Event, Resource, Stats};

// Note: These tests require making Resource fields public
// Current implementation has private fields, so these are examples of the target design

#[test]
fn given_resource_has_capacity_when_consumer_requests_then_immediately_acquired() {
    // GIVEN: Resource with capacity 2, no active consumers
    let mut resource = Resource::new(0, 2);

    // Verify initial state
    assert_eq!(resource.consumer_count, 0);
    assert_eq!(resource.consumer_queue.len(), 0);

    // WHEN: Consumer 42 requests resource at t=10
    let response = resource.act(10, &Event::ResourceRequested(0, 42));

    // THEN: Resource immediately grants access via ResourceAcquired event
    assert_eq!(response.events.len(), 1, "Should emit exactly one event");
    assert_eq!(response.agents.len(), 0, "Should not spawn new agents");

    match &response.events[0] {
        (t, Event::ResourceAcquired(rid, cid, req_t)) => {
            assert_eq!(*t, 10, "Event should occur at current time");
            assert_eq!(*rid, 0, "Should be for resource 0");
            assert_eq!(*cid, 42, "Should be for consumer 42");
            assert_eq!(*req_t, 10, "Request time should match current time");
        }
        _ => panic!("Expected ResourceAcquired event, got {:?}", response.events[0]),
    }

    // THEN: Resource internal state updated
    assert_eq!(resource.consumer_count, 1, "One consumer now active");
    assert_eq!(resource.consumer_queue.len(), 0, "No queuing necessary");
    assert_eq!(resource.stats.acquired_count, 1, "Stats updated");
    assert_eq!(resource.stats.arrival_count, 1, "Arrival tracked");
}

#[test]
fn given_full_resource_when_consumer_requests_then_queued_not_acquired() {
    // GIVEN: Resource at full capacity (1 consumer active)
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));

    // Verify precondition
    assert_eq!(resource.consumer_count, 1);

    // WHEN: Second consumer requests resource at t=15
    let response = resource.act(15, &Event::ResourceRequested(0, 2));

    // THEN: No immediate acquisition - consumer is queued
    assert_eq!(response.events.len(), 0, "Should not emit any events");
    assert_eq!(resource.consumer_queue.len(), 1, "Consumer should be queued");
    assert_eq!(resource.consumer_count, 1, "Consumer count unchanged");
    assert!(
        resource.consumers_active.contains(&2),
        "Consumer 2 should be marked active (waiting)"
    );

    // THEN: Queue contains correct consumer info
    let (queued_cid, queued_t) = resource.consumer_queue.front().unwrap();
    assert_eq!(*queued_cid, 2, "Consumer 2 should be in queue");
    assert_eq!(*queued_t, 15, "Request time should be recorded");
}

#[test]
fn given_queued_consumers_when_resource_released_then_next_consumer_acquires() {
    // GIVEN: Resource at capacity with queue
    let mut resource = Resource::new(0, 1);

    // Consumer 1 acquires at t=10
    resource.act(10, &Event::ResourceRequested(0, 1));

    // Consumers 2 and 3 are queued
    resource.act(15, &Event::ResourceRequested(0, 2));
    resource.act(17, &Event::ResourceRequested(0, 3));

    // Verify preconditions
    assert_eq!(resource.consumer_count, 1);
    assert_eq!(resource.consumer_queue.len(), 2);

    // WHEN: First consumer releases at t=25
    let response = resource.act(25, &Event::ResourceReleased(0, 1, 10));

    // THEN: Next queued consumer (id=2, requested at t=15) acquires resource
    assert_eq!(response.events.len(), 1, "Should emit ResourceAcquired event");

    match &response.events[0] {
        (t, Event::ResourceAcquired(rid, cid, req_t)) => {
            assert_eq!(*t, 25, "Acquisition happens at release time");
            assert_eq!(*cid, 2, "Consumer 2 is next in queue");
            assert_eq!(*req_t, 15, "Original request time preserved");
        }
        _ => panic!("Expected ResourceAcquired"),
    }

    // THEN: Resource state updated correctly
    assert_eq!(resource.consumer_count, 1, "Still at capacity");
    assert_eq!(resource.consumer_queue.len(), 1, "One consumer dequeued");
    assert!(!resource.consumers_active.contains(&2), "Consumer 2 no longer waiting");
    assert!(resource.consumers_active.contains(&3), "Consumer 3 still waiting");

    // THEN: Wait time statistics tracked
    // Consumer 2 waited from t=15 to t=25 = 10 time units
    assert_eq!(resource.stats.wait_sum, 10, "Wait time should be tracked");
}

#[test]
fn given_queued_consumer_when_request_expires_then_removed_from_queue() {
    // GIVEN: Resource at capacity with consumer 2 in queue
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1)); // occupies resource
    resource.act(15, &Event::ResourceRequested(0, 2)); // queued

    // Verify preconditions
    assert_eq!(resource.consumer_queue.len(), 1);
    assert!(resource.consumers_active.contains(&2));

    // WHEN: Consumer 2's request expires at t=35 (after 20 time units of waiting)
    let response = resource.act(35, &Event::ResourceRequestExpired(0, 2, 15));

    // THEN: Consumer removed from active set
    assert_eq!(response.events.len(), 0, "No events emitted");
    assert!(!resource.consumers_active.contains(&2), "Consumer 2 marked as expired");

    // THEN: Expiry statistics tracked
    assert_eq!(resource.stats.expiry_count, 1);
    assert_eq!(resource.stats.wait_sum, 20, "Wait time until expiry tracked");

    // WHEN: Resource is released
    let response = resource.act(40, &Event::ResourceReleased(0, 1, 10));

    // THEN: No consumer acquires (expired consumer skipped in queue)
    assert_eq!(response.events.len(), 0, "Expired consumer not granted access");
    assert_eq!(resource.consumer_count, 0, "Resource now idle");
}

#[test]
fn given_multiple_queued_consumers_when_one_expires_then_others_remain() {
    // GIVEN: Resource at capacity with multiple queued consumers
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1)); // occupies
    resource.act(15, &Event::ResourceRequested(0, 2)); // queued
    resource.act(17, &Event::ResourceRequested(0, 3)); // queued
    resource.act(19, &Event::ResourceRequested(0, 4)); // queued

    // WHEN: Middle consumer (3) expires
    resource.act(30, &Event::ResourceRequestExpired(0, 3, 17));

    // THEN: Consumer 3 removed from active set
    assert!(!resource.consumers_active.contains(&3));
    assert!(resource.consumers_active.contains(&2), "Consumer 2 still active");
    assert!(resource.consumers_active.contains(&4), "Consumer 4 still active");

    // WHEN: Resource is released
    let response = resource.act(35, &Event::ResourceReleased(0, 1, 10));

    // THEN: Consumer 2 (first in queue) acquires, skipping expired consumer 3
    match &response.events[0] {
        (_, Event::ResourceAcquired(_, cid, _)) => {
            assert_eq!(*cid, 2, "Consumer 2 should acquire, not expired consumer 3");
        }
        _ => panic!("Expected ResourceAcquired"),
    }
}

#[test]
fn given_resource_for_different_id_when_event_received_then_ignored() {
    // GIVEN: Resource with id=0
    let mut resource = Resource::new(0, 1);

    // WHEN: Events for resource id=1 are broadcast
    let response1 = resource.act(10, &Event::ResourceRequested(1, 42));
    let response2 = resource.act(15, &Event::ResourceReleased(1, 42, 10));
    let response3 = resource.act(20, &Event::ResourceRequestExpired(1, 42, 10));

    // THEN: All events ignored (no response)
    assert_eq!(response1.events.len(), 0);
    assert_eq!(response2.events.len(), 0);
    assert_eq!(response3.events.len(), 0);

    // THEN: Resource state unchanged
    assert_eq!(resource.consumer_count, 0);
    assert_eq!(resource.stats.arrival_count, 0);
}

#[test]
fn given_resource_when_multiple_operations_then_stats_accumulate_correctly() {
    // GIVEN: Resource with capacity 2
    let mut resource = Resource::new(0, 2);

    // WHEN: Multiple consumers arrive and use the resource
    // t=10: Consumer 1 arrives and acquires
    resource.act(10, &Event::ResourceRequested(0, 1));
    // t=15: Consumer 2 arrives and acquires
    resource.act(15, &Event::ResourceRequested(0, 2));
    // t=20: Consumer 3 arrives but must queue
    resource.act(20, &Event::ResourceRequested(0, 3));
    // t=30: Consumer 1 releases (used for 20 time units)
    resource.act(30, &Event::ResourceReleased(0, 1, 10));
    // t=35: Consumer 2 releases (used for 20 time units)
    resource.act(35, &Event::ResourceReleased(0, 2, 15));

    // THEN: Statistics accumulated correctly
    assert_eq!(resource.stats.arrival_count, 3, "Three arrivals");
    assert_eq!(resource.stats.acquired_count, 3, "Three acquisitions");
    assert_eq!(resource.stats.expiry_count, 0, "No expirations");
    assert_eq!(resource.stats.consume_sum, 40, "Total consume time: 20 + 20");
    // Consumer 3 waited from t=20 to t=30 = 10 time units
    assert_eq!(resource.stats.wait_sum, 10, "Total wait time");
}
