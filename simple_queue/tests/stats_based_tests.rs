// Stats-based testing: All assertions via public Stats interface
// No private field access required - proper encapsulation maintained

use des::Agent;
use simple_queue::{Event, Resource, Stats};

// Helper to extract ResourceStats from Stats enum
fn get_resource_stats(stats: &Stats) -> &simple_queue::ResourceStats {
    match stats {
        Stats::ResourceStats(rs) => rs,
        _ => panic!("Expected ResourceStats"),
    }
}

// ============================================================================
// Level 1: Unit Tests - Individual Agent Behavior
// ============================================================================

#[test]
fn given_empty_resource_when_consumer_requests_then_immediately_acquired() {
    // GIVEN: Resource with capacity 2, no active consumers
    let mut resource = Resource::new(0, 2);

    // Verify initial state via Stats (not field access)
    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.current_consumer_count, 0);
    assert_eq!(stats.current_queue_length, 0);
    assert!(!stats.is_at_capacity());

    // WHEN: Consumer 42 requests resource at t=10
    let response = resource.act(10, &Event::ResourceRequested(0, 42));

    // THEN: Immediate acquisition event
    assert_eq!(response.events.len(), 1);
    match &response.events[0] {
        (t, Event::ResourceAcquired(rid, cid, req_t)) => {
            assert_eq!(*t, 10);
            assert_eq!(*rid, 0);
            assert_eq!(*cid, 42);
            assert_eq!(*req_t, 10);
        }
        _ => panic!("Expected ResourceAcquired event"),
    }

    // THEN: State change observable via Stats
    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.current_consumer_count, 1);
    assert_eq!(stats.current_queue_length, 0);
    assert_eq!(stats.total_arrivals, 1);
    assert_eq!(stats.total_acquired, 1);
    assert!(!stats.is_at_capacity());
}

#[test]
fn given_full_resource_when_consumer_requests_then_queued_not_acquired() {
    // GIVEN: Resource at full capacity
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));

    // Verify at capacity via Stats
    let stats = get_resource_stats(&resource.stats());
    assert!(stats.is_at_capacity());
    assert_eq!(stats.utilization(), 1.0);

    // WHEN: Second consumer requests
    let response = resource.act(15, &Event::ResourceRequested(0, 2));

    // THEN: No immediate acquisition
    assert_eq!(response.events.len(), 0);

    // THEN: Consumer queued (observable via Stats, not field access)
    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.current_queue_length, 1);
    assert_eq!(stats.current_active_requests, 1);
    assert!(stats.has_queue());
    assert_eq!(stats.total_arrivals, 2);
    assert_eq!(stats.total_acquired, 1);
}

#[test]
fn given_queued_consumers_when_resource_released_then_next_acquires() {
    // GIVEN: Resource at capacity with queue
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));
    resource.act(15, &Event::ResourceRequested(0, 2));
    resource.act(17, &Event::ResourceRequested(0, 3));

    // Verify queue state via Stats
    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.current_consumer_count, 1);
    assert_eq!(stats.current_queue_length, 2);

    // WHEN: First consumer releases at t=25
    let response = resource.act(25, &Event::ResourceReleased(0, 1, 10));

    // THEN: Next consumer acquires
    assert_eq!(response.events.len(), 1);
    match &response.events[0] {
        (t, Event::ResourceAcquired(rid, cid, req_t)) => {
            assert_eq!(*t, 25);
            assert_eq!(*rid, 0);
            assert_eq!(*cid, 2);  // First in queue
            assert_eq!(*req_t, 15);
        }
        _ => panic!("Expected ResourceAcquired"),
    }

    // THEN: Queue reduced, wait time tracked (all via Stats)
    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.current_consumer_count, 1);
    assert_eq!(stats.current_queue_length, 1);  // Was 2, now 1
    assert_eq!(stats.total_acquired, 2);
    assert_eq!(stats.total_wait_time, 10);  // Consumer 2 waited 15->25
}

#[test]
fn given_queued_consumer_when_request_expires_then_removed() {
    // GIVEN: Resource at capacity with queued consumer
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));
    resource.act(15, &Event::ResourceRequested(0, 2));

    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.current_queue_length, 1);
    assert_eq!(stats.current_active_requests, 1);

    // WHEN: Consumer 2's request expires
    let response = resource.act(35, &Event::ResourceRequestExpired(0, 2, 15));

    // THEN: No events emitted
    assert_eq!(response.events.len(), 0);

    // THEN: Expiry tracked in Stats
    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.total_expired, 1);
    assert_eq!(stats.total_wait_time, 20);  // Waited 15->35

    // WHEN: Resource released
    let response = resource.act(40, &Event::ResourceReleased(0, 1, 10));

    // THEN: No consumer acquires (expired consumer skipped)
    assert_eq!(response.events.len(), 0);

    // THEN: Resource now idle (via Stats)
    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.current_consumer_count, 0);
    assert_eq!(stats.current_queue_length, 0);
}

#[test]
fn given_multiple_queued_when_one_expires_then_others_remain() {
    // GIVEN: Resource with multiple queued consumers
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));
    resource.act(15, &Event::ResourceRequested(0, 2));
    resource.act(17, &Event::ResourceRequested(0, 3));
    resource.act(19, &Event::ResourceRequested(0, 4));

    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.current_queue_length, 3);

    // WHEN: Middle consumer expires
    resource.act(30, &Event::ResourceRequestExpired(0, 3, 17));

    // THEN: One expiry recorded
    let stats = get_resource_stats(&resource.stats());
    assert_eq!(stats.total_expired, 1);
    // Queue still has 3 items (expired consumer not yet removed from queue)
    // But active requests decreased
    assert_eq!(stats.current_active_requests, 2);  // Was 3, now 2

    // WHEN: Resource released
    let response = resource.act(35, &Event::ResourceReleased(0, 1, 10));

    // THEN: Consumer 2 (first non-expired) acquires
    match &response.events[0] {
        (_, Event::ResourceAcquired(_, cid, _)) => {
            assert_eq!(*cid, 2);
        }
        _ => panic!("Expected ResourceAcquired"),
    }
}

// ============================================================================
// Level 2: State Transitions
// ============================================================================

#[test]
fn resource_state_transitions_from_empty_to_queuing() {
    let mut resource = Resource::new(0, 2);

    // State 1: Empty
    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.current_consumer_count, 0);
    assert!(!s.is_at_capacity());

    // Transition: First consumer
    resource.act(10, &Event::ResourceRequested(0, 1));

    // State 2: One consumer
    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.current_consumer_count, 1);
    assert_eq!(s.utilization(), 0.5);
    assert!(!s.is_at_capacity());

    // Transition: Second consumer
    resource.act(15, &Event::ResourceRequested(0, 2));

    // State 3: At capacity
    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.current_consumer_count, 2);
    assert!(s.is_at_capacity());
    assert_eq!(s.utilization(), 1.0);

    // Transition: Third consumer
    resource.act(20, &Event::ResourceRequested(0, 3));

    // State 4: Queueing begins
    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.current_consumer_count, 2);
    assert_eq!(s.current_queue_length, 1);
    assert!(s.has_queue());

    // Transition: First consumer releases
    resource.act(30, &Event::ResourceReleased(0, 1, 10));

    // State 5: Queue processed
    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.current_consumer_count, 2);  // Still at capacity
    assert_eq!(s.current_queue_length, 0);    // Queue cleared
    assert!(!s.has_queue());
}

#[test]
fn resource_metrics_accumulate_over_multiple_consumers() {
    let mut resource = Resource::new(0, 1);

    // Consumer 1: uses resource for 30 time units
    resource.act(10, &Event::ResourceRequested(0, 1));
    resource.act(40, &Event::ResourceReleased(0, 1, 10));

    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.total_consume_time, 30);
    assert_eq!(s.total_released, 1);
    assert_eq!(s.avg_consume_time(), Some(30.0));

    // Consumer 2: uses resource for 50 time units
    resource.act(45, &Event::ResourceRequested(0, 2));
    resource.act(95, &Event::ResourceReleased(0, 2, 45));

    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.total_consume_time, 80);
    assert_eq!(s.total_released, 2);
    assert_eq!(s.avg_consume_time(), Some(40.0));

    // Consumer 3: uses for 20 time units
    resource.act(100, &Event::ResourceRequested(0, 3));
    resource.act(120, &Event::ResourceReleased(0, 3, 100));

    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.total_consume_time, 100);
    assert_eq!(s.total_released, 3);
    assert_eq!(s.avg_consume_time(), Some(100.0 / 3.0));
}

#[test]
fn wait_time_statistics_tracked_correctly() {
    let mut resource = Resource::new(0, 1);

    // Occupy resource
    resource.act(10, &Event::ResourceRequested(0, 1));

    // Queue three consumers
    resource.act(15, &Event::ResourceRequested(0, 2));
    resource.act(20, &Event::ResourceRequested(0, 3));
    resource.act(25, &Event::ResourceRequested(0, 4));

    // Consumer 1 releases at t=50
    resource.act(50, &Event::ResourceReleased(0, 1, 10));

    // Consumer 2 waited from 15 to 50 = 35 time units
    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.total_wait_time, 35);
    assert_eq!(s.avg_wait_time(), Some(35.0));

    // Consumer 2 releases at t=80
    resource.act(80, &Event::ResourceReleased(0, 2, 15));

    // Consumer 3 waited from 20 to 80 = 60 time units
    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.total_wait_time, 95);  // 35 + 60
    assert_eq!(s.avg_wait_time(), Some(95.0 / 2.0));
}

// ============================================================================
// Level 3: Event Sourcing Pattern
// ============================================================================

#[test]
fn event_sequence_replay_with_stats_snapshots() {
    let mut resource = Resource::new(0, 1);

    // Event log with expected Stats predicates
    let scenarios = vec![
        (
            (10, Event::ResourceRequested(0, 1)),
            |s: &simple_queue::ResourceStats| {
                s.current_consumer_count == 1
                && s.total_acquired == 1
                && !s.has_queue()
            },
        ),
        (
            (15, Event::ResourceRequested(0, 2)),
            |s: &simple_queue::ResourceStats| {
                s.current_queue_length == 1
                && s.current_active_requests == 1
                && s.total_arrivals == 2
            },
        ),
        (
            (20, Event::ResourceRequested(0, 3)),
            |s: &simple_queue::ResourceStats| {
                s.current_queue_length == 2
                && s.total_arrivals == 3
            },
        ),
        (
            (30, Event::ResourceRequestExpired(0, 2, 15)),
            |s: &simple_queue::ResourceStats| {
                s.total_expired == 1
                && s.total_wait_time == 15
                && s.current_queue_length == 2  // Still in queue
            },
        ),
        (
            (40, Event::ResourceReleased(0, 1, 10)),
            |s: &simple_queue::ResourceStats| {
                s.total_released == 1
                && s.current_queue_length == 0  // Consumer 3 acquired
                && s.total_acquired == 2  // Consumer 1 and 3
            },
        ),
    ];

    // Replay events and verify Stats at each step
    for ((t, event), predicate) in scenarios {
        resource.act(t, &event);
        let stats = get_resource_stats(&resource.stats());
        assert!(
            predicate(stats),
            "Stats predicate failed at t={} for event {:?}",
            t, event
        );
    }
}

// ============================================================================
// Stats Semantic Methods
// ============================================================================

#[test]
fn stats_semantic_methods_provide_clear_queries() {
    let mut resource = Resource::new(0, 3);

    // is_at_capacity()
    resource.act(10, &Event::ResourceRequested(0, 1));
    assert!(!get_resource_stats(&resource.stats()).is_at_capacity());

    resource.act(15, &Event::ResourceRequested(0, 2));
    resource.act(20, &Event::ResourceRequested(0, 3));
    assert!(get_resource_stats(&resource.stats()).is_at_capacity());

    // has_queue()
    assert!(!get_resource_stats(&resource.stats()).has_queue());

    resource.act(25, &Event::ResourceRequested(0, 4));
    assert!(get_resource_stats(&resource.stats()).has_queue());

    // utilization()
    assert_eq!(get_resource_stats(&resource.stats()).utilization(), 1.0);

    resource.act(30, &Event::ResourceReleased(0, 1, 10));
    // Still at capacity (consumer 4 acquired)
    assert_eq!(get_resource_stats(&resource.stats()).utilization(), 1.0);

    resource.act(35, &Event::ResourceReleased(0, 2, 15));
    resource.act(40, &Event::ResourceReleased(0, 3, 20));
    resource.act(45, &Event::ResourceReleased(0, 4, 25));

    // All released
    assert_eq!(get_resource_stats(&resource.stats()).utilization(), 0.0);
}

#[test]
fn stats_avg_methods_handle_edge_cases() {
    let resource = Resource::new(0, 1);

    // No acquisitions yet
    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.avg_wait_time(), None);
    assert_eq!(s.avg_consume_time(), None);

    // After some activity
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));
    resource.act(50, &Event::ResourceReleased(0, 1, 10));

    let s = get_resource_stats(&resource.stats());
    assert_eq!(s.avg_consume_time(), Some(40.0));
    // No wait time yet (immediate acquisition)
    assert_eq!(s.avg_wait_time(), Some(0.0));
}

// ============================================================================
// Demonstration: No Field Access Needed
// ============================================================================

#[test]
fn complete_test_using_only_stats_interface() {
    // This test verifies complex behavior using ONLY the Stats interface
    // No private field access, no public fields required

    let mut resource = Resource::new(0, 2);

    // Scenario: Multiple consumers competing for limited resource
    let events = vec![
        Event::ResourceRequested(0, 1),  // t=10
        Event::ResourceRequested(0, 2),  // t=15
        Event::ResourceRequested(0, 3),  // t=20 (queued)
        Event::ResourceRequested(0, 4),  // t=25 (queued)
        Event::ResourceRequestExpired(0, 3, 20),  // t=30 (expires)
        Event::ResourceReleased(0, 1, 10),  // t=40 (c4 acquires)
        Event::ResourceReleased(0, 2, 15),  // t=50
        Event::ResourceReleased(0, 4, 30),  // t=60
    ];

    let times = vec![10, 15, 20, 25, 30, 40, 50, 60];

    for (t, event) in times.iter().zip(events.iter()) {
        resource.act(*t, event);
    }

    // Verify final state using only Stats
    let final_stats = get_resource_stats(&resource.stats());

    // All consumers processed
    assert_eq!(final_stats.total_arrivals, 4);
    assert_eq!(final_stats.total_acquired, 3);  // 1, 2, 4 (3 expired)
    assert_eq!(final_stats.total_expired, 1);   // Consumer 3
    assert_eq!(final_stats.total_released, 3);

    // Resource is idle
    assert_eq!(final_stats.current_consumer_count, 0);
    assert_eq!(final_stats.current_queue_length, 0);
    assert!(!final_stats.has_queue());
    assert_eq!(final_stats.utilization(), 0.0);

    // Time metrics tracked
    assert!(final_stats.total_wait_time > 0);
    assert!(final_stats.total_consume_time > 0);
    assert!(final_stats.avg_wait_time().is_some());
    assert!(final_stats.avg_consume_time().is_some());

    // All verified without touching private fields!
}
