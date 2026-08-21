// ruviz Ruby binding — native extension entry point.
//
// Phase 1: extension skeleton only. This wires up the `Ruviz` module and a
// hello-world method so the whole Ruby<->Rust build/load pipeline can be
// verified before the ruviz plot API is bound in Phase 2.

use magnus::{function, prelude::*, Error, Ruby};

/// Version of this native binding (kept in sync with lib/ruviz/version.rb).
const BINDING_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Smoke-test hook: proves the extension loaded and Rust code runs.
fn hello() -> String {
    format!("ruviz-ruby native extension loaded (v{BINDING_VERSION})")
}

// `name` must match the loaded library basename so Ruby finds `Init_ruviz`.
#[magnus::init(name = "ruviz")]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Ruviz")?;
    module.define_singleton_method("_hello", function!(hello, 0))?;
    Ok(())
}
