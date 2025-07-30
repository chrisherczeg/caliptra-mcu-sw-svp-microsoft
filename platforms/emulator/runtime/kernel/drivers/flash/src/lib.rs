// Licensed under the Apache-2.0 license.

#![cfg_attr(target_arch = "riscv32", no_std)]

#[cfg(target_arch = "riscv32")]
pub mod flash_ctrl;
pub mod flash_storage_to_pages;
pub mod hil;
