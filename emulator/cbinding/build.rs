/*++

Licensed under the Apache-2.0 license.

File Name:

    build.rs

Abstract:

    Build script to generate C headers for the emulator bindings.

--*/

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .with_style(cbindgen::Style::Both)
        .with_include_guard("EMULATOR_CBINDING_H")
        .with_pragma_once(true)
        .with_documentation(true)
        .with_parse_deps(false)
        .include_item("EmulatorError")
        .include_item("CStepAction")
        .include_item("CEmulator")
        .include_item("CEmulatorConfig")
        .include_item("emulator_get_size")
        .include_item("emulator_get_alignment")
        .include_item("emulator_init")
        .include_item("emulator_step")
        .include_item("emulator_destroy")
        .include_item("emulator_get_uart_output")
        .include_item("get_pc")
        .include_item("trigger_exit_request")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("emulator_cbinding.h");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
