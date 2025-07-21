/*++

Licensed under the Apache-2.0 license.

File Name:

    example.c

Abstract:

    Example C program demonstrating how to use the emulator C bindings.

--*/

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdalign.h>
#include "emulator_cbinding.h"

int main(int argc, char* argv[]) {
    // Check if we have enough arguments
    if (argc < 6) {
        printf("Usage: %s <rom_path> <firmware_path> <caliptra_rom_path> <caliptra_firmware_path> <soc_manifest_path>\n", argv[0]);
        return 1;
    }

    // Get the size and alignment requirements for the emulator
    size_t emulator_size = emulator_get_size();
    size_t emulator_alignment = emulator_get_alignment();
    
    printf("Emulator requires %zu bytes with %zu-byte alignment\n", emulator_size, emulator_alignment);

    // Allocate aligned memory for the emulator
    void* emulator_memory = aligned_alloc(emulator_alignment, emulator_size);
    if (!emulator_memory) {
        printf("Failed to allocate memory for emulator\n");
        return 1;
    }

    // Configure the emulator
    CEmulatorConfig config = {
        .rom_path = argv[1],
        .firmware_path = argv[2],
        .caliptra_rom_path = argv[3],
        .caliptra_firmware_path = argv[4],
        .soc_manifest_path = argv[5],
        .otp_path = NULL,
        .log_dir_path = NULL,
        .gdb_port = 0,
        .i3c_port = 0,
        .trace_instr = 0,
        .stdin_uart = 0,
        .manufacturing_mode = 0,
        .capture_uart_output = 1, // Enable UART output capture
        .vendor_pk_hash = NULL,
        .owner_pk_hash = NULL,
        .streaming_boot_path = NULL,
        .primary_flash_image_path = NULL,
        .secondary_flash_image_path = NULL,
        .hw_revision_major = 2,
        .hw_revision_minor = 0,
        .hw_revision_patch = 0,
        // Memory layout overrides (0 = use defaults)
        .rom_offset = 0,            // Use default ROM offset
        .rom_size = 0,              // Use default ROM size
        .uart_offset = 0,           // Use default UART offset
        .uart_size = 0,             // Use default UART size
        .ctrl_offset = 0,           // Use default control offset
        .ctrl_size = 0,             // Use default control size
        .spi_offset = 0,            // Use default SPI offset
        .spi_size = 0,              // Use default SPI size
        .sram_offset = 0,           // Use default SRAM offset
        .sram_size = 0,             // Use default SRAM size
        .pic_offset = 0,            // Use default PIC offset
        .external_test_sram_offset = 0,  // Use default external test SRAM offset
        .external_test_sram_size = 0,    // Use default external test SRAM size
        .dccm_offset = 0,           // Use default DCCM offset
        .dccm_size = 0,             // Use default DCCM size
        .i3c_offset = 0,            // Use default I3C offset
        .i3c_size = 0,              // Use default I3C size
        .primary_flash_offset = 0,  // Use default primary flash offset
        .primary_flash_size = 0,    // Use default primary flash size
        .secondary_flash_offset = 0, // Use default secondary flash offset
        .secondary_flash_size = 0,  // Use default secondary flash size
        .mci_offset = 0,            // Use default MCI offset
        .mci_size = 0,              // Use default MCI size
        .dma_offset = 0,            // Use default DMA offset
        .dma_size = 0,              // Use default DMA size
        .mbox_offset = 0,           // Use default mailbox offset
        .mbox_size = 0,             // Use default mailbox size
        .soc_offset = 0,            // Use default SoC offset
        .soc_size = 0,              // Use default SoC size
        .otp_offset = 0,            // Use default OTP offset
        .otp_size = 0,              // Use default OTP size
        .lc_offset = 0,             // Use default LC offset
        .lc_size = 0,               // Use default LC size
    };

    // Initialize the emulator
    EmulatorError init_result = emulator_init((CEmulator*)emulator_memory, &config);
    if (init_result != Success) {
        printf("Failed to initialize emulator: %d\n", init_result);
        free(emulator_memory);
        return 1;
    }

    printf("Emulator initialized successfully\n");

    // Run the emulator for a limited number of steps
    const int max_steps = 1000;
    int step_count = 0;
    CStepAction action;
    
    printf("Starting emulator execution...\n");
    
    do {
        action = emulator_step((CEmulator*)emulator_memory);
        step_count++;
        
        if (step_count % 100 == 0) {
            printf("Executed %d steps, action: %d\n", step_count, action);
        }
        
        // Check for UART output periodically
        if (step_count % 50 == 0) {
            char uart_buffer[1024];
            int uart_bytes = emulator_get_uart_output((CEmulator*)emulator_memory, uart_buffer, sizeof(uart_buffer));
            if (uart_bytes > 0) {
                printf("UART Output (%d bytes): %s\n", uart_bytes, uart_buffer);
            }
        }
        
    } while (action == Continue && step_count < max_steps);

    printf("Emulator stopped after %d steps with action: %d\n", step_count, action);

    // Get final UART output
    char final_uart_buffer[4096];
    int final_uart_bytes = emulator_get_uart_output((CEmulator*)emulator_memory, final_uart_buffer, sizeof(final_uart_buffer));
    if (final_uart_bytes > 0) {
        printf("Final UART Output (%d bytes): %s\n", final_uart_bytes, final_uart_buffer);
    }

    // Clean up
    emulator_destroy((CEmulator*)emulator_memory);
    free(emulator_memory);

    printf("Emulator cleaned up successfully\n");
    return 0;
}
