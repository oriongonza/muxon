//! Compile-time test: planner types and verb constants exist.
use resurreccion_planner::{NodeId, Plan, verbs};

#[test]
fn empty_plan_has_no_nodes() {
    let plan = Plan::empty();
    assert!(plan.nodes.is_empty());
}

#[test]
fn node_id_is_unique() {
    let a = NodeId::new();
    let b = NodeId::new();
    assert_ne!(a, b);
}

#[test]
fn verb_constants_nonempty() {
    assert!(!verbs::CAPTURE_LAYOUT.is_empty());
    assert!(!verbs::RESTORE_LAYOUT.is_empty());
}
