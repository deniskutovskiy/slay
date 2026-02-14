use crate::common::TestHarness;
use slay_core::{canonical_key, EdgeConfig, Link};

#[test]
fn test_high_latency_timeout() {
    let mut h = TestHarness::new();
    let client = h.add_client(1, 10.0);
    client.config.write().unwrap().timeout = 100;
    h.add_server(2, "S1", 10, 1, 10);

    // Given: Latency (200ms) > Timeout (100ms)
    let mut link = Link::default();
    link.get_config_mut(1, 2).latency_us = 200_000;
    h.connect_with_link(1, 2, link);

    h.start();
    h.run_for(1000);

    // Then: All requests fail
    assert_eq!(
        h.sim.success_count, 0,
        "Should have 0 successes due to timeout"
    );
    assert!(
        h.sim.failure_count > 0,
        "Should have failures due to timeout"
    );
}

#[test]
fn test_latency_delay_success() {
    let mut h = TestHarness::new();
    let client = h.add_client(1, 10.0);
    client.config.write().unwrap().timeout = 1000;
    h.add_server(2, "S1", 10, 1, 10);

    // Given: 100ms latency each way (200ms RTT)
    let mut link = Link::default();
    link.get_config_mut(1, 2).latency_us = 100_000;
    link.get_config_mut(2, 1).latency_us = 100_000;
    h.connect_with_link(1, 2, link);

    h.start();
    h.run_for(1000);

    // Then: Requests succeed AND take at least 200ms (100+100)
    assert!(h.sim.success_count > 0, "Should succeed despite latency");
    let p_min = h.sim.get_percentile(1.0, 500_000).unwrap_or(0);
    assert!(p_min >= 200_000, "RTT {} should be >= 200ms latency", p_min);
}

#[test]
fn test_packet_loss_50_percent() {
    let mut h = TestHarness::new();
    h.add_client(1, 100.0);
    h.add_server(2, "S1", 10, 100, 100);

    // Given: 50% packet loss
    let mut link = Link::default();
    link.get_config_mut(1, 2).packet_loss_rate = 0.5;
    h.connect_with_link(1, 2, link);

    h.start();
    h.run_for(1000);

    // Then: Success rate is within statistical bounds of 50%
    let total = h.sim.success_count + h.sim.failure_count;
    let rate = h.sim.success_count as f64 / total as f64;
    assert!(
        (rate - 0.5).abs() < 0.15,
        "Success rate {:.2} should be close to 0.5",
        rate
    );
}

#[test]
fn test_jitter_variability() {
    let mut h = TestHarness::new();
    h.add_client(1, 100.0);
    h.add_server(2, "S1", 1, 1, 100);

    // Given: 10ms Latency + 50ms Jitter
    let base = 10_000;
    let jitter = 50_000;
    let mut link = Link::default();
    *link.get_config_mut(1, 2) = EdgeConfig {
        latency_us: base,
        jitter_us: jitter,
        packet_loss_rate: 0.0,
    };
    h.connect_with_link(1, 2, link);

    h.start();
    h.run_for(2000);

    // Then: RTT varies by ~Jitter amount
    // Expected Min RTT ~ 20ms (10 fwd + 10 back)
    // Expected Max RTT ~ 70ms (10+50 fwd + 10 back)
    let p1 = h.sim.get_percentile(1.0, 500_000).unwrap_or(0);
    let p99 = h.sim.get_percentile(99.0, 500_000).unwrap_or(0);
    let spread = p99 - p1;

    assert!(
        (spread as i64 - jitter as i64).abs() < 15_000,
        "Spread {} should be close to jitter {}",
        spread,
        jitter
    );
}

#[test]
fn test_dynamic_edge_update() {
    let mut h = TestHarness::new();
    let client = h.add_client(1, 10.0);
    client.config.write().unwrap().timeout = 50;
    h.add_server(2, "S1", 10, 1, 10);
    h.connect(1, 2); // Default link

    // Phase 1: Fast (Default)
    h.start();
    h.run_for(500);
    let success_p1 = h.sim.success_count;
    assert!(success_p1 > 0);

    // Phase 2: Slow (Update to 100ms > 50ms timeout)
    let mut link = Link::default();
    link.get_config_mut(1, 2).latency_us = 100_000;
    h.sim.links.insert(canonical_key(1, 2), link);

    h.run_for(500);
    let new_successes = h.sim.success_count - success_p1;
    assert!(
        new_successes <= 1,
        "Should stop succeeding after latency increase"
    );

    // Phase 3: Fast Again
    h.sim.links.insert(canonical_key(1, 2), Link::default());
    h.run_for(500);
    assert!(
        h.sim.success_count > success_p1 + new_successes,
        "Should recover"
    );
}

#[test]
fn test_default_edge_behavior() {
    let mut h = TestHarness::new();
    h.add_client(1, 10.0);
    h.add_server(2, "S1", 10, 1, 10);
    h.connect(1, 2);

    h.start();
    h.run_for(1000);

    // Then: Requests succeed AND obey default physics (10ms each way = 20ms RTT min)
    assert!(h.sim.success_count > 0, "Default link should work");
    let p_min = h.sim.get_percentile(1.0, 500_000).unwrap_or(0);
    assert!(
        p_min >= 20_000,
        "Default RTT {} should be >= 20ms latency",
        p_min
    );
}

#[test]
fn test_asymmetric_jitter() {
    let mut h = TestHarness::new();
    h.add_client(1, 100.0);
    h.add_server(2, "S1", 1, 1, 100);

    // Given: Fwd(10ms + 40ms Jitter), Back(100ms Stable)
    let mut link = Link::default();
    *link.get_config_mut(1, 2) = EdgeConfig {
        latency_us: 10_000,
        jitter_us: 40_000,
        packet_loss_rate: 0.0,
    };
    *link.get_config_mut(2, 1) = EdgeConfig {
        latency_us: 100_000,
        jitter_us: 0,
        packet_loss_rate: 0.0,
    };
    h.connect_with_link(1, 2, link);

    h.start();
    h.run_for(2000);

    // Then: Min RTT ~110ms, Max RTT ~150ms
    let p_min = h.sim.get_percentile(1.0, 500_000).unwrap_or(0);
    let p_max = h.sim.get_percentile(99.0, 500_000).unwrap_or(0);

    assert!(p_min >= 100_000, "Min RTT should include return latency");
    assert!(p_max >= 140_000, "Max RTT should include jitter");
}
