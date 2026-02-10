use crate::common::TestHarness;

#[test]
fn test_aws_realism() {
    let mut h = TestHarness::new();

    h.add_server(2, "Server", 50, 2, 100);
    h.add_client(1, 20.0);
    h.set_target(1, 2);

    h.start();
    h.run_for(5000);
    assert!(h.sla() > 99.0);
    assert!(h.p99() < 150);
}

#[test]
fn test_saturation_and_recovery() {
    let mut h = TestHarness::new();
    h.add_server(2, "Server", 100, 1, 5);
    let client = h.add_client(1, 50.0);
    h.set_target(1, 2);

    h.start();
    h.run_for(5000);
    assert!(h.sla() < 90.0);

    {
        let mut config = client.config.write().unwrap();
        config.arrival_rate = 2.0;
    }

    h.sim.reset_stats();
    if let Some(c) = h.sim.components.get_mut(&2) {
        c.reset_internal_stats();
    }

    h.start();
    h.run_for(20000);
    assert!(h.sla() > 95.0);
}
