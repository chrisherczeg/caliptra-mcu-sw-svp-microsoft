/*++

Licensed under the Apache-2.0 license.

File Name:

    example.c

Abstract:

    Example C program demonstrating how to use the emulator C bindings,
    including both normal mode and GDB integration.

--*/

#include "emulator_cbinding.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char *argv[]) {
    // Parse command line arguments
    unsigned int gdb_port = 0;
    if (argc >= 3 && strcmp(argv[1], "--gdb") == 0) {
        gdb_port = (unsigned int)atoi(argv[2]);
        printf("GDB mode enabled on port %u\n", gdb_port);
    }

    // Get memory requirements and allocate
    size_t emulator_size = emulator_size_required();
    void* memory = aligned_alloc(8, emulator_size);
    if (!memory) {
        fprintf(stderr, "Failed to allocate memory\n");
        return -1;
    }

    printf("Allocated %zu bytes for emulator\n", emulator_size);

    // Configure the emulator
    EmulatorArgs args = {
        .soc_address_hi = 0x00000000,
        .soc_address_lo = 0x40000000,
        .soc_size = 0x800000,
        .uc_address_hi = 0x00000000,
        .uc_address_lo = 0x50000000,
        .uc_size = 0x10000,
        .soc_manifest = "test_manifest.bin",
        .gdb_port = gdb_port
    };

    // Initialize emulator
    EmulatorError result = emulator_init((struct CEmulator*)memory, &args);
    if (result != Success) {
        fprintf(stderr, "Failed to initialize emulator: %d\n", result);
        free(memory);
        return -1;
    }

    printf("Emulator initialized successfully\n");

    // Check if we're in GDB mode
    if (emulator_is_gdb_mode((struct CEmulator*)memory)) {
        unsigned int port = emulator_get_gdb_port((struct CEmulator*)memory);
        printf("GDB server available on port %u\n", port);
        printf("Connect with: gdb -ex 'target remote :%u'\n", port);
        
        if (gdb_port != 0) {
            // Demonstrate C-controlled stepping in GDB mode
            printf("Running 10 steps under C control while GDB server is available...\n");
            for (int i = 0; i < 10; i++) {
                EmulatorError step_result = emulator_step((struct CEmulator*)memory);
                if (step_result != Success) {
                    printf("Step %d failed with error %d\n", i, step_result);
                    break;
                }
                printf("Completed step %d\n", i + 1);
            }
            
            printf("Now starting GDB server (this will block until GDB disconnects)\n");
            printf("You can connect with GDB and take control of execution\n");
            
            // Hand control over to GDB
            EmulatorError gdb_result = emulator_run_gdb_server((struct CEmulator*)memory);
            if (gdb_result == Success) {
                printf("GDB session completed successfully\n");
            } else {
                printf("GDB session failed with error %d\n", gdb_result);
            }
        }
    } else {
        // Normal mode - step the emulator
        printf("Running emulator in normal mode...\n");
        
        for (int i = 0; i < 1000; i++) {
            EmulatorError step_result = emulator_step((struct CEmulator*)memory);
            
            if (step_result == Success) {
                // Continue running
                if (i % 100 == 0) {
                    printf("Completed %d steps\n", i);
                }
            } else if (step_result == StepComplete) {
                printf("Emulator finished execution after %d steps\n", i);
                break;
            } else {
                printf("Step failed with error %d after %d steps\n", step_result, i);
                break;
            }
            
            // Check for UART output every 10 steps
            if (i % 10 == 0) {
                char output[256];
                int output_len = emulator_get_uart_output((struct CEmulator*)memory, output, sizeof(output) - 1);
                if (output_len > 0) {
                    output[output_len] = '\0';
                    printf("UART: %s", output);
                }
            }
        }
    }

    // Final UART output check
    char final_output[1024];
    int final_len = emulator_get_uart_output((struct CEmulator*)memory, final_output, sizeof(final_output) - 1);
    if (final_len > 0) {
        final_output[final_len] = '\0';
        printf("Final UART output: %s", final_output);
    }

    // Clean up
    emulator_destroy((struct CEmulator*)memory);
    free(memory);
    
    printf("Emulator cleaned up\n");
    return 0;
}
