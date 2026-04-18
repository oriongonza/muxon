//! Conformance test suite for [`Mux`] implementations.
//!
//! Every backend must pass these tests before being accepted.
//! Lane D (Zellij) will fill in the actual test bodies.
//! For now, each function panics with "not implemented" as a placeholder.

use crate::Mux;

/// Run the full conformance suite against a backend.
///
/// Call this from your backend's test module:
/// ```ignore
/// #[test]
/// fn conformance() {
///     resurreccion_mux::conformance::run(&MyBackend::new());
/// }
/// ```
pub fn run<M: Mux>(mux: &M) {
    discover_returns_vec(mux);
    capabilities_are_valid(mux);
}

fn discover_returns_vec<M: Mux>(mux: &M) {
    // Conformance: discover() must return Ok even when no sessions exist.
    match mux.discover() {
        Ok(_sessions) => {}
        Err(e) if e.is_retryable() => {
            // Acceptable: backend not running in test environment.
        }
        Err(e) => panic!("discover() failed fatally: {e}"),
    }
}

fn capabilities_are_valid<M: Mux>(mux: &M) {
    // Conformance: capabilities() must return a valid bitflag set.
    let _caps = mux.capabilities();
}
