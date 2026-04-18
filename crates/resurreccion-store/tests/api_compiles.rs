//! Compile-time test: Store API exists and method signatures are as expected.
use resurreccion_store::Store;

#[test]
fn store_open_signature_exists() {
    // This won't actually open a DB in tests — just verifies the type exists.
    let _: fn(&str) -> anyhow::Result<Store> = Store::open;
}
