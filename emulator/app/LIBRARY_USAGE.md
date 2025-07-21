# Emulator Library Usage Example

This example demonstrates how to use the emulator crate as a library.

```rust
use emulator::{Emulator, EmulatorArgs, EMULATOR_RUNNING};

fn main() {
    // Create emulator args programmatically
    let args = EmulatorArgs {
        rom: "path/to/rom.bin".into(),
        firmware: "path/to/firmware.bin".into(),
        // ... other args
    };
    
    // Create emulator instance
    let emulator = Emulator::from_args(args, false).expect("Failed to create emulator");
    
    // Use the emulator as needed
    // Access global state via EMULATOR_RUNNING static
}
```

## Library Structure

The emulator crate now provides both:

1. **Binary target**: `emulator` - The standalone emulator executable
2. **Library target**: `emulator` - Library for reuse by other crates

### Public API

The library exports:
- `Emulator` struct and its methods
- `EmulatorArgs` for configuration  
- Global variables: `EMULATOR_RUNNING`, `MCU_RUNTIME_STARTED`
- Utility function: `wait_for_runtime_start()`
- All submodules: `gdb`, `tests`, `doe_mbox_fsm`, etc.

### Usage in Other Crates

Add to your `Cargo.toml`:
```toml
[dependencies]
emulator = { path = "../emulator/app" }
```

Then import and use:
```rust
use emulator::{Emulator, EmulatorArgs};
```
