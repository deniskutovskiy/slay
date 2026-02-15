use crate::common::TestHarness;
use slay_core::components::load_balancer::BalancingStrategy;
use slay_core::LoadBalancer;
use std::sync::Arc;

#[test]
fn test_load_balancer_round_robin() {
    let mut h = TestHarness::new();
    h.add_client(1, 100.0);
    let lb = LoadBalancer::new("LB");
    let lb_config = Arc::clone(&lb.config);
    h.add(2, Box::new(lb));
    h.add_server(3, "S1", 10, 100, 100);
    h.add_server(4, "S2", 10, 100, 100);
    h.connect(1, 2);
    h.connect(2, 3);
    h.connect(2, 4);
    {
        let mut cfg = lb_config.write().unwrap();
        cfg.strategy = BalancingStrategy::RoundRobin;
    }
    h.start();
    h.run_for(1000);
    assert!(h.sim.success_count > 90);
    let s1_rps = h.sim.components.get(&3).unwrap().active_throughput();
    let s2_rps = h.sim.components.get(&4).unwrap().active_throughput();
    let diff = (s1_rps - s2_rps).abs();
    assert!(diff <= 5.0);
}

#[test]
fn test_load_balancer_least_connections() {
    let mut h = TestHarness::new();

    h.add_client(1, 200.0);
    let lb = LoadBalancer::new("LB");
    let lb_config = Arc::clone(&lb.config);
    h.add(2, Box::new(lb));

    h.add_server(3, "Slow", 500, 100, 100);
    h.add_server(4, "Fast", 1, 100, 100);

    h.connect(1, 2);
    h.connect(2, 3);
    h.connect(2, 4);

    {
        let mut cfg = lb_config.write().unwrap();
        cfg.strategy = BalancingStrategy::LeastConnections;
    }

    h.start();
    h.run_for(1000);

    let s3_rps = h.sim.components.get(&3).unwrap().active_throughput();
    let s4_rps = h.sim.components.get(&4).unwrap().active_throughput();

    assert!(
        s4_rps > s3_rps * 5.0,
        "Least Connections must favor the Fast server when Slow is bogged down"
    );
}
