use crate::common::TestHarness;

#[test]
fn test_chained_response_path() {
    let mut h = TestHarness::new();

    h.add_server(3, "Server", 50, 1, 10);
    h.add_server(2, "Proxy", 10, 1, 10);
    h.add_client(1, 10.0);

    h.set_target(1, 2);
    h.set_target(2, 3);

    h.start();
    h.run_for(2000);

    assert!(h.sim.success_count > 0);
    assert!(h.sim.get_percentile(50.0, 5000) > 80);
}

#[test]
fn test_strict_timeout() {
    let mut h = TestHarness::new();
    h.add_server(2, "SlowServer", 1000, 1, 10);

    let client = h.add_client(1, 10.0);
    {
        let mut config = client.config.write().unwrap();
        config.timeout = 100;
    }
    h.set_target(1, 2);

    h.start();
    h.run_for(2000);
    assert!(h.sim.failure_count > 0);
    assert_eq!(h.sim.success_count, 0);
}
