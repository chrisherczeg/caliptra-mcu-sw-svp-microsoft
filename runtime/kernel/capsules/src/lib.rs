// Licensed under the Apache-2.0 license

#![cfg_attr(target_arch = "riscv32", no_std)]
#![forbid(unsafe_code)]

pub mod test;

pub mod doe;
pub mod mailbox;
pub mod mctp;
