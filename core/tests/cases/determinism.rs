use crate::common::TestHarness;

#[test]
fn test_determinism_across_runs() {
    let seed = 12345;

    // Run 1
    let mut h1 = TestHarness::new_with_seed(seed);
    h1.add_client(1, 100.0);
    h1.add_server(2, "Server", 10, 10, 100);
    h1.connect(1, 2);
    h1.start();
    h1.run_for(100); // 100ms

    // Run 2
    let mut h2 = TestHarness::new_with_seed(seed);
    h2.add_client(1, 100.0);
    h2.add_server(2, "Server", 10, 10, 100);
    h2.connect(1, 2);
    h2.start();
    h2.run_for(100); // 100ms

    // Check global stats
    assert_eq!(
        h1.sim.success_count, h2.sim.success_count,
        "Success count mismatch"
    );
    assert_eq!(
        h1.sim.failure_count, h2.sim.failure_count,
        "Failure count mismatch"
    );
    assert_eq!(
        h1.sim.latencies.len(),
        h2.sim.latencies.len(),
        "Latency count mismatch"
    );

    // Check exact latencies
    for (i, (t1, l1)) in h1.sim.latencies.iter().enumerate() {
        let (t2, l2) = h2.sim.latencies[i];
        assert_eq!(*t1, t2, "Latency timestamp mismatch at index {}", i);
        assert_eq!(*l1, l2, "Latency value mismatch at index {}", i);
    }
}

#[test]
fn test_determinism_with_different_seeds() {
    // Should produce DIFFERENT results (statistically likely)
    let mut h1 = TestHarness::new_with_seed(100);
    h1.add_client(1, 100.0);
    h1.add_server(2, "Server", 10, 10, 100);
    h1.connect(1, 2);
    h1.start();
    h1.run_for(200);

    let mut h2 = TestHarness::new_with_seed(200);
    h2.add_client(1, 100.0);
    h2.add_server(2, "Server", 10, 10, 100);
    h2.connect(1, 2);
    h2.start();
    h2.run_for(200);

    // It is theoretically possible they match if 0 requests processed or pure chance,
    // but with high RPS and jitter, they should diverge.
    let latency_sum1: u64 = h1.sim.latencies.iter().map(|(_, l)| l).sum();
    let latency_sum2: u64 = h2.sim.latencies.iter().map(|(_, l)| l).sum();

    assert_ne!(
        latency_sum1, latency_sum2,
        "Different seeds should produce different results"
    );
}
