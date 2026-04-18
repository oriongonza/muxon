//! Compile-time test: Mux trait and associated types exist and are object-safe.
use resurreccion_mux::{Capability, LayoutSpec, Mux, MuxError};

/// Verify the trait is dyn-compatible (object-safe).
#[allow(clippy::used_underscore_items)]
fn _assert_dyn_mux(_: &dyn Mux) {}

#[test]
fn capability_flags_are_bitflags() {
    let none = Capability::empty();
    let plugin = Capability::PLUGIN_EMBEDDING;
    assert!(!none.contains(plugin));
}

#[test]
#[allow(clippy::used_underscore_items)]
fn mux_error_is_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<MuxError>();
}

#[test]
fn layout_spec_default() {
    let spec = LayoutSpec::default();
    assert!(spec.panes.is_empty());
}
