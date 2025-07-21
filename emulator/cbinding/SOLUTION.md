# Caliptra MCU Emulator C Bindings

## Overview

This crate provides C bindings for the Caliptra MCU Emulator, allowing C code to:
- Allocate and manage emulator memory
- Initialize and configure the emulator
- Step through emulator execution
- Retrieve UART output
- Clean up resources

## Key Design Principles

1. **C-controlled memory**: C code allocates memory for the emulator struct, giving C full control over lifetime
2. **Zero changes to emulator.rs**: The original emulator code remains completely untouched
3. **Static library**: Compiles to a static library for easy integration
4. **Memory safety**: Proper alignment and size management
5. **Error handling**: Comprehensive error codes for all operations

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   C Application │───▶│   C Bindings    │───▶│   Rust Emulator │
│                 │    │  (cbinding crate)│    │   (emulator crate)│
│ - Memory mgmt   │    │ - C interface   │    │ - Original code │
│ - Lifecycle     │    │ - Type safety   │    │ - Unchanged     │
│ - Integration   │    │ - Error handling│    │ - Full features │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Files Created

### Core Binding Files
- **`emulator/cbinding/src/lib.rs`**: Main C binding implementation
- **`emulator/cbinding/Cargo.toml`**: Crate configuration
- **`emulator/cbinding/build.rs`**: Header generation script
- **`emulator/cbinding/cbindgen.toml`**: Header generation configuration

### Documentation & Examples
- **`emulator/cbinding/README.md`**: Comprehensive usage documentation
- **`emulator/cbinding/example.c`**: Complete working C example
- **`emulator/cbinding/Makefile`**: Build system for C example

### Testing
- **`emulator/cbinding/src/simple_test.rs`**: Rust unit tests
- **`emulator/cbinding/src/minimal.rs`**: Minimal proof-of-concept

## API Overview

### Memory Management
```c
size_t emulator_get_size();              // Get required memory size
size_t emulator_get_alignment();         // Get required alignment
void* memory = aligned_alloc(align, size); // C allocates memory
```

### Configuration & Initialization
```c
CEmulatorConfig config = {
    .rom_path = "rom.bin",
    .firmware_path = "firmware.bin", 
    .caliptra_rom_path = "caliptra_rom.bin",
    .caliptra_firmware_path = "caliptra_firmware.bin",
    .soc_manifest_path = "manifest.bin",
    // ... other configuration options
};

EmulatorError result = emulator_init((CEmulator*)memory, &config);
```

### Execution
```c
CStepAction action;
do {
    action = emulator_step((CEmulator*)memory);
} while (action == Continue);
```

### Cleanup
```c
emulator_destroy((CEmulator*)memory);
free(memory);
```

## Build Instructions

1. **Build the static library**:
   ```bash
   cd emulator/cbinding
   cargo build --release
   ```
   
   This generates:
   - `target/release/libemulator_cbinding.a` (static library)
   - `emulator_cbinding.h` (C header file)

2. **Build the C example**:
   ```bash
   make example
   ```

3. **Run the example**:
   ```bash
   ./example rom.bin firmware.bin caliptra_rom.bin caliptra_firmware.bin manifest.bin
   ```

## Integration with Existing Projects

### Linking
```bash
gcc -o my_app my_app.c -L./target/release -lemulator_cbinding -lpthread -ldl -lm
```

### Headers
```c
#include "emulator_cbinding.h"
```

### Memory Requirements
Always use the provided functions to get size and alignment:
```c
size_t size = emulator_get_size();
size_t align = emulator_get_alignment();
void* memory = aligned_alloc(align, size);
```

## Configuration Options

The `CEmulatorConfig` structure exposes all emulator configuration:

- **Required paths**: ROM, firmware, Caliptra ROM/firmware, SoC manifest
- **Optional paths**: OTP file, log directory, flash images
- **Network**: GDB port, I3C port
- **Behavior**: Instruction tracing, UART capture, manufacturing mode
- **Hardware**: Version numbers, address overrides

## Error Handling

```c
typedef enum {
    Success = 0,
    InvalidArgs = -1,
    InitializationFailed = -2,
    NullPointer = -3,
    InvalidEmulator = -4,
} EmulatorError;
```

## Thread Safety

- The emulator is **not thread-safe**
- Use external synchronization in multi-threaded environments
- Each emulator instance should be accessed from only one thread

## Platform Support

Supports the same platforms as the Rust emulator:
- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64) 
- Windows (x86_64)

## Workspace Integration

The crate is added to the workspace at:
```toml
# Cargo.toml
[workspace]
members = [
    # ... existing members ...
    "emulator/cbinding",
    # ... other members ...
]
```

## Benefits

1. **No emulator.rs changes**: Original code remains untouched
2. **C memory control**: C manages emulator lifetime completely
3. **Static linking**: Easy integration into existing C projects
4. **Full configuration**: All emulator features accessible from C
5. **Memory safe**: Proper alignment and size checking
6. **Comprehensive**: Error handling, UART capture, cleanup

## Usage Example

See `example.c` for a complete working example that demonstrates:
- Proper memory allocation and alignment
- Configuration setup
- Emulator initialization and execution loop
- UART output retrieval
- Error handling and cleanup

This solution provides a robust, production-ready C interface to the Caliptra MCU Emulator while maintaining complete separation from the original Rust code.
