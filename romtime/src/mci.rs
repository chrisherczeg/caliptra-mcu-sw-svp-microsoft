// Licensed under the Apache-2.0 license

use crate::static_ref::StaticRef;
use registers_generated::mci;
use tock_registers::interfaces::{Readable, Writeable};

/// MCU Reset Reason
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum McuResetReason {
    /// Cold Boot - Power-on reset (no bits set)
    ColdBoot,

    /// Warm Reset - MCU reset while power maintained
    WarmReset,

    /// Firmware Boot Update - First firmware update after MCI reset
    FirmwareBootUpdate,

    /// Firmware Hitless Update - Second or later firmware update
    FirmwareHitlessUpdate,

    /// Multiple bits set - invalid state
    Invalid,
}

pub struct Mci {
    pub registers: StaticRef<mci::regs::Mci>,
}

impl Mci {
    pub const fn new(registers: StaticRef<mci::regs::Mci>) -> Self {
        Mci { registers }
    }

    pub fn device_lifecycle_state(&self) -> mci::bits::SecurityState::DeviceLifecycle::Value {
        self.registers
            .mci_reg_security_state
            .read_as_enum(mci::bits::SecurityState::DeviceLifecycle)
            .unwrap_or(mci::bits::SecurityState::DeviceLifecycle::Value::DeviceUnprovisioned)
    }

    pub fn security_state(&self) -> u32 {
        self.registers.mci_reg_security_state.get()
    }

    pub fn caliptra_boot_go(&self) {
        self.registers.mci_reg_cptra_boot_go.set(1);
    }

    pub fn flow_status(&self) -> u32 {
        self.registers.mci_reg_fw_flow_status.get()
    }

    pub fn hw_flow_status(&self) -> u32 {
        self.registers.mci_reg_hw_flow_status.get()
    }

    pub fn set_nmi_vector(&self, nmi_vector: u32) {
        self.registers.mci_reg_mcu_nmi_vector.set(nmi_vector);
    }

    pub fn configure_wdt(&self, wdt1_timeout: u32, wdt2_timeout: u32) {
        // Set WDT1 period.
        self.registers.mci_reg_wdt_timer1_timeout_period[0].set(wdt1_timeout);
        self.registers.mci_reg_wdt_timer1_timeout_period[1].set(0);

        // Set WDT2 period. Fire immediately after WDT1 expiry
        self.registers.mci_reg_wdt_timer2_timeout_period[0].set(wdt2_timeout);
        self.registers.mci_reg_wdt_timer2_timeout_period[1].set(0);

        // Enable WDT1 only. WDT2 is automatically scheduled (since it is disabled) on WDT1 expiry.
        self.registers.mci_reg_wdt_timer1_ctrl.set(1); // Timer1Restart
        self.registers.mci_reg_wdt_timer1_en.set(1); // Timer1En
    }

    pub fn disable_wdt(&self) {
        self.registers.mci_reg_wdt_timer1_en.set(0); // Timer1En CLEAR
    }

    /// Read the reset reason register value
    pub fn reset_reason(&self) -> u32 {
        self.registers.mci_reg_reset_reason.get()
    }

    /// Get the reset reason as an enum
    pub fn reset_reason_enum(&self) -> McuResetReason {
        let warm_reset = self
            .registers
            .mci_reg_reset_reason
            .read(mci::bits::ResetReason::WarmReset)
            != 0;
        let fw_boot_upd = self
            .registers
            .mci_reg_reset_reason
            .read(mci::bits::ResetReason::FwBootUpdReset)
            != 0;
        let fw_hitless_upd = self
            .registers
            .mci_reg_reset_reason
            .read(mci::bits::ResetReason::FwHitlessUpdReset)
            != 0;

        match (warm_reset, fw_boot_upd, fw_hitless_upd) {
            (false, false, false) => McuResetReason::ColdBoot,
            (true, false, false) => McuResetReason::WarmReset,
            (false, true, false) => McuResetReason::FirmwareBootUpdate,
            (false, false, true) => McuResetReason::FirmwareHitlessUpdate,
            _ => McuResetReason::Invalid,
        }
    }

    /// Check if this is a cold reset (power-on reset)
    pub fn is_cold_reset(&self) -> bool {
        self.reset_reason_enum() == McuResetReason::ColdBoot
    }

    /// Check if this is a warm reset
    pub fn is_warm_reset(&self) -> bool {
        self.reset_reason_enum() == McuResetReason::WarmReset
    }

    /// Check if this is a firmware boot update reset
    pub fn is_fw_boot_update_reset(&self) -> bool {
        self.reset_reason_enum() == McuResetReason::FirmwareBootUpdate
    }

    /// Check if this is a firmware hitless update reset
    pub fn is_fw_hitless_update_reset(&self) -> bool {
        self.reset_reason_enum() == McuResetReason::FirmwareHitlessUpdate
    }
}
