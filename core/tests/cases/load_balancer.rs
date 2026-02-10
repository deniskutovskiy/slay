use crate::common::TestHarness;
use slay_core::*;

#[test]
fn test_load_balancer_round_robin() {
    let mut h = TestHarness::new();
    
    // Topology: Client -> LB -> [Srv1, Srv2]
    h.add_server(2, "Srv1", 50, 1, 10);
    h.add_server(3, "Srv2", 50, 1, 10);
    
    let mut lb = LoadBalancer::new("LB");
    lb.add_target(2);
    lb.add_target(3);
    h.add(4, Box::new(lb));
    
    h.add_client(1, 20.0);
    h.set_target(1, 4);

    h.start();
    h.run_for(2000);

    // We expect ~40 requests total, distributed roughly equally
    assert!(h.sim.success_count > 30);
    
    // Check internal distribution by getting handles
    if let Some(_lb_any) = h.sim.components.get(&4) {
        // Here we'd ideally use a handle, but for a quick check we can see
        // if both servers processed something
        let s1_reqs = h.sim.components.get(&2).unwrap().active_throughput();
        let s2_reqs = h.sim.components.get(&3).unwrap().active_throughput();
        
        println!("S1 RPS: {}, S2 RPS: {}", s1_reqs, s2_reqs);
        assert!(s1_reqs > 0.0);
        assert!(s2_reqs > 0.0);
    }
}
