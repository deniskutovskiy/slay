use crate::common::TestHarness;
use slay_core::components::load_balancer::BalancingStrategy;
use slay_core::components::server::Server;
use slay_core::LoadBalancer;
use std::sync::Arc;

#[test]
fn test_retry_on_failure() {
    let mut h = TestHarness::new();

    // 1. Setup Client
    h.add_client(1, 10.0); // 10 RPS

    // 2. Setup Load Balancer with Retries
    let lb = LoadBalancer::new("LB");
    let lb_config = Arc::clone(&lb.config);
    h.add(2, Box::new(lb));

    // 3. Setup Servers
    // Server 3: Failing (100% failure rate)
    let s3 = Server::new("BadServer", 100, 100, 100);
    let s3_config = Arc::clone(&s3.config); // Retain config handle
    h.add(3, Box::new(s3));

    // Configure failure
    s3_config.write().unwrap().failure_probability = 1.0;

    // Server 4: Healthy
    h.add_server(4, "GoodServer", 10, 100, 100);

    // 4. Connect
    h.connect(1, 2);
    h.connect(2, 3);
    h.connect(2, 4);

    // 5. Configure LB
    {
        let mut cfg = lb_config.write().unwrap();
        cfg.strategy = BalancingStrategy::RoundRobin;
        cfg.max_retries = 1;
        cfg.retry_backoff_ms = 10;
    }

    h.start();
    h.run_for(1000); // Run for 1 second (approx 10 requests)

    let success_rate = h.sla();
    println!("SLA: {}%", success_rate);

    assert!(h.sim.success_count > 0);
    assert!(
        success_rate > 90.0,
        "SLA should be high because retries rescue failures from BadServer"
    );

    // Check total retries metric on LB
    let lb_snapshot = h.sim.components.get(&2).unwrap().get_visual_snapshot();
    let total_retries = lb_snapshot["total_retries"].as_u64().unwrap_or(0);
    println!("Total Retries: {}", total_retries);
    assert!(total_retries > 0, "LB should have performed retries");
}

#[test]
fn test_retry_exhaustion() {
    let mut h = TestHarness::new();
    h.add_client(1, 10.0);

    let lb = LoadBalancer::new("LB");
    let lb_config = Arc::clone(&lb.config);
    h.add(2, Box::new(lb));

    // Both servers fail
    let s3 = Server::new("Bad1", 100, 100, 100);
    let s3_config = Arc::clone(&s3.config);
    h.add(3, Box::new(s3));

    let s4 = Server::new("Bad2", 100, 100, 100);
    let s4_config = Arc::clone(&s4.config);
    h.add(4, Box::new(s4));

    // Configure failure probability
    s3_config.write().unwrap().failure_probability = 1.0;
    s4_config.write().unwrap().failure_probability = 1.0;

    h.connect(1, 2);
    h.connect(2, 3);
    h.connect(2, 4);

    {
        let mut cfg = lb_config.write().unwrap();
        cfg.max_retries = 1;
    }
    h.start();
    h.run_for(500);

    let success_rate = h.sla();
    println!("SLA: {}%", success_rate);

    // Should be 0% success (or very close to 0)
    assert!(
        success_rate < 5.0,
        "Should have near 0% success as all servers fail"
    );

    let lb_snapshot = h.sim.components.get(&2).unwrap().get_visual_snapshot();
    let total_retries = lb_snapshot["total_retries"].as_u64().unwrap_or(0);
    assert!(
        total_retries > 0,
        "Retries should happen even if they eventually fail"
    );
}

#[test]
fn test_retry_budget_exhaustion() {
    let mut h = TestHarness::new();
    // High RPS to build budget quickly based on new requests.
    h.add_client(1, 10.0);

    let lb = LoadBalancer::new("LB");
    let lb_config = Arc::clone(&lb.config);
    h.add(2, Box::new(lb));

    // Server 3: Failing
    let s3 = Server::new("Bad", 100, 100, 100);
    let s3_config = Arc::clone(&s3.config);
    h.add(3, Box::new(s3));

    s3_config.write().unwrap().failure_probability = 1.0;

    // Server 4: Healthy
    h.add_server(4, "Good", 10, 100, 100);

    h.connect(1, 2);
    h.connect(2, 3);
    h.connect(2, 4);

    {
        let mut cfg = lb_config.write().unwrap();
        cfg.max_retries = 1;
        cfg.retry_budget_ratio = 0.1; // Strict budget: 10%
    }

    h.start();
    // Run for 50 requests (5 sec at 10 RPS)
    // 50 requests -> budget += 5.0 tokens (max).
    // RR strategy: 25 requests go to "Bad", 25 to "Good".
    // 25 failures from "Bad".
    // We attempt to retry.
    // Cost: 1.0 per retry.
    // Budget allows ~15 retries.
    // 10 retries dropped.
    h.run_for(5000);

    let snapshot = h.sim.components.get(&2).unwrap().get_visual_snapshot();
    let total_retries = snapshot["total_retries"].as_u64().unwrap_or(0);

    println!("Total Retries: {}", total_retries);

    assert!(
        total_retries < 25,
        "Budget should prevent some retries (Expected ~15, Got {})",
        total_retries
    );
    assert!(
        total_retries >= 10,
        "Budget should allow some retries (Expected ~15, Got {})",
        total_retries
    );
}
