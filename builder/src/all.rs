// Licensed under the Apache-2.0 license

use anyhow::{bail, Result};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{
    write::{FileOptions, SimpleFileOptions},
    ZipWriter,
};

#[derive(Default)]
pub struct FirmwareBinaries {
    pub caliptra_rom: Vec<u8>,
    pub caliptra_fw: Vec<u8>,
    pub mcu_rom: Vec<u8>,
    pub mcu_runtime: Vec<u8>,
    pub soc_manifest: Vec<u8>,
}

impl FirmwareBinaries {
    const CALIPTRA_ROM_NAME: &'static str = "caliptra_rom.bin";
    const CALIPTRA_FW_NAME: &'static str = "caliptra_fw.bin";
    const MCU_ROM_NAME: &'static str = "mcu_rom.bin";
    const MCU_RUNTIME_NAME: &'static str = "mcu_runtime.bin";
    const SOC_MANIFEST_NAME: &'static str = "soc_manifest.bin";

    pub fn read_from_zip(path: &PathBuf) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut zip = zip::ZipArchive::new(file)?;
        let mut binaries = FirmwareBinaries::default();

        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;

            match name.as_str() {
                Self::CALIPTRA_ROM_NAME => binaries.caliptra_rom = data,
                Self::CALIPTRA_FW_NAME => binaries.caliptra_fw = data,
                Self::MCU_ROM_NAME => binaries.mcu_rom = data,
                Self::MCU_RUNTIME_NAME => binaries.mcu_runtime = data,
                Self::SOC_MANIFEST_NAME => binaries.soc_manifest = data,
                _ => continue,
            }
        }

        Ok(binaries)
    }
}

/// Build Caliptra ROM and firmware bundle, MCU ROM and runtime, and SoC manifest, and package them all together in a ZIP file.
pub fn all_build(
    output: Option<&str>,
    platform: Option<&str>,
    use_dccm_for_stack: bool,
    dccm_offset: Option<u32>,
    dccm_size: Option<u32>,
) -> Result<()> {
    // TODO: use temp files
    let platform = platform.unwrap_or("emulator");
    let mcu_rom = crate::rom_build(Some(platform), "")?;
    let memory_map = match platform {
        "emulator" => &mcu_config_emulator::EMULATOR_MEMORY_MAP,
        "fpga" => &mcu_config_fpga::FPGA_MEMORY_MAP,
        _ => bail!("Unknown platform: {:?}", platform),
    };
    let mcu_runtime = &crate::runtime_build_with_apps_cached(
        &[],
        None,
        false,
        Some(platform),
        Some(memory_map),
        use_dccm_for_stack,
        dccm_offset,
        dccm_size,
        None,
    )?;

    let fpga = platform == "fpga";
    let mut caliptra_builder =
        crate::CaliptraBuilder::new(fpga, None, None, None, None, Some(mcu_runtime.into()), None);
    let caliptra_rom = caliptra_builder.get_caliptra_rom()?;
    let caliptra_fw = caliptra_builder.get_caliptra_fw()?;
    let vendor_pk_hash = caliptra_builder.get_vendor_pk_hash()?;
    println!("Vendor PK hash: {:x?}", vendor_pk_hash);
    let soc_manifest = caliptra_builder.get_soc_manifest()?;

    let default_path = crate::target_dir().join("all-fw.zip");
    let path = output.map(Path::new).unwrap_or(&default_path);
    println!("Creating ZIP file: {}", path.display());
    let file = std::fs::File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644)
        .last_modified_time(zip::DateTime::try_from(chrono::Local::now().naive_local())?);

    add_to_zip(
        &caliptra_rom,
        FirmwareBinaries::CALIPTRA_ROM_NAME,
        &mut zip,
        options,
    )?;
    add_to_zip(
        &caliptra_fw,
        FirmwareBinaries::CALIPTRA_FW_NAME,
        &mut zip,
        options,
    )?;
    add_to_zip(
        &PathBuf::from(mcu_rom),
        FirmwareBinaries::MCU_ROM_NAME,
        &mut zip,
        options,
    )?;
    add_to_zip(
        &PathBuf::from(mcu_runtime),
        FirmwareBinaries::MCU_RUNTIME_NAME,
        &mut zip,
        options,
    )?;
    add_to_zip(
        &soc_manifest,
        FirmwareBinaries::SOC_MANIFEST_NAME,
        &mut zip,
        options,
    )?;
    zip.finish()?;

    Ok(())
}

fn add_to_zip(
    input_file: &PathBuf,
    name: &str,
    zip: &mut ZipWriter<std::fs::File>,
    options: FileOptions<'_, ()>,
) -> Result<()> {
    let data = std::fs::read(input_file)?;
    println!("Adding {}: {} bytes", name, data.len());
    zip.start_file(name, options)?;
    zip.write_all(&data)?;
    Ok(())
}
