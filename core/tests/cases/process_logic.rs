use crate::common::TestHarness;
use slay_core::LoadBalancer;

#[test]
fn test_chained_response_path() {
    let mut h = TestHarness::new();
    h.add_client(1, 10.0);
    h.add_server(2, "S1", 100, 1, 10);
    h.connect(1, 2);

    h.start();
    h.run_for(1000);

    assert!(h.sim.success_count > 0);
    assert!(h.sim.get_percentile(50.0, 5_000_000).unwrap_or(0) > 80_000);
}

#[test]
fn test_strict_timeout() {
    let mut h = TestHarness::new();
    let client_handle = h.add_client(1, 50.0);
    {
        let mut cfg = client_handle.config.write().unwrap();
        cfg.timeout = 50;
    }

    h.add_server(2, "Slow", 200, 1, 10);
    h.connect(1, 2);

    h.start();
    h.run_for(1000);

    assert!(h.sim.failure_count > 0);
}

#[test]
fn test_rps_accounting_precision() {
    let mut h = TestHarness::new();

    h.add_client(1, 100.0);
    h.add(2, Box::new(LoadBalancer::new("LB")));
    h.add_server(3, "S1", 10, 100, 100);

    h.connect(1, 2);
    h.connect(2, 3);

    h.start();
    h.run_for(1000);

    let total = h.sim.success_count + h.sim.failure_count;

    assert!(
        total > 90 && total < 110,
        "RPS accounting must be precise, got {}",
        total
    );
}
