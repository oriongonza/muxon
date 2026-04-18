//! Integration tests for `ZellijMux`.

use resurreccion_mux::Mux;
use resurreccion_zellij::ZellijMux;

fn zellij_available() -> bool {
    std::process::Command::new("zellij")
        .arg("--version")
        .output()
        .is_ok()
}

#[test]
fn capabilities_advertised_correctly() {
    let mux = ZellijMux::new();
    let caps = mux.capabilities();

    // Zellij 0.1.0: PluginEmbedding is not supported.
    assert!(
        !caps.contains(resurreccion_mux::Capability::PLUGIN_EMBEDDING),
        "PLUGIN_EMBEDDING should not be set for Zellij 0.1.0"
    );
}

#[test]
fn discover_returns_vec() {
    if !zellij_available() {
        return;
    }

    let mux = ZellijMux::new();
    match mux.discover() {
        Ok(_sessions) => {
            // Success: discover() returned a vec (even if empty).
        }
        Err(e) if e.is_retryable() => {
            // Acceptable: zellij backend not running.
        }
        Err(e) => panic!("discover() failed fatally: {e}"),
    }
}

#[test]
fn conformance_suite_passes() {
    if !zellij_available() {
        return;
    }

    let mux = ZellijMux::new();
    resurreccion_mux::conformance::run(&mux);
}
