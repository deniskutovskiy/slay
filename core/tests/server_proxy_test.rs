mod common;
use common::TestHarness;

#[test]
fn test_server_proxy_chain() {
    let mut h = TestHarness::new();

    // 1. Setup Client (Node 1) -> sends requests
    let client = h.add_client(1, 10.0);
    drop(client.config.read().unwrap());

    // 2. Setup Server A (Node 2) -> acts as proxy
    let _s1 = h.add_server(2, "Proxy A", 10, 100, 100);

    // 3. Setup Server B (Node 3) -> final destination
    let _s2 = h.add_server(3, "Backend B", 10, 100, 100);

    // 4. Connect Client -> Server A -> Server B
    h.connect(1, 2);
    h.connect(2, 3);

    // 5. Start simulation
    h.start();

    // 6. Run for a short duration.
    h.run_for(100);

    assert!(h.sla() > 0.0, "SLA should be positive");
    let _p99 = h.p99();

    // If we reach here, check success count.

    // If we reach here, check success count.
    // With 10 RPS for 100ms => ~1 request.
    // We expect at least one success if things are working.
    assert!(
        h.sim.success_count > 0,
        "Simulation should have processed at least one request successfully"
    );
}
