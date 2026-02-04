use crate::ds::dust_core::{
    audio::DummyBackend,
    cpu::interpreter::Interpreter,
    ds_slot,
    emu::{self, Emu},
    flash::Flash,
    spi::{self, firmware},
    SaveContents,
    Model,
    utils::{Bytes, BoxedByteSlice},
};
use crate::ds::dust_soft_2d::sync::Renderer as Renderer2d;
use crate::ds::soft_renderer_3d;
use crate::ds::rtc_backend::RtcBackend;
use crate::ds::audio::DsAudioBackend;

use std::path::Path;
use std::fs;
use std::convert::TryFrom;
use std::convert::TryInto;
use sdl2::AudioSubsystem;

pub struct DsEmulator {
    pub emu: Emu<Interpreter>,
}

impl DsEmulator {
    pub fn new(rom_path: &Path, arm7_bios: Option<&Path>, arm9_bios: Option<&Path>, firmware: Option<&Path>, audio_subsystem: &AudioSubsystem, save_path: &Path) -> Result<Self, String> {
        let rom_data = fs::read(rom_path).map_err(|e| format!("Failed to read ROM: {}", e))?;
        // Ensure size is power of 2 and sufficient
        let len = rom_data.len().next_power_of_two().max(0x8000); // 32KB min?
        let mut rom_boxed_byte_slice = BoxedByteSlice::new_zeroed(len);
        rom_boxed_byte_slice[..rom_data.len()].copy_from_slice(&rom_data);

        // Load BIOS if provided
        let arm7_bios_data = if let Some(path) = arm7_bios {
            let data = fs::read(path).map_err(|e| format!("Failed to read ARM7 BIOS: {}", e))?;
            let arr: [u8; 16384] = data.try_into().map_err(|_| "Invalid ARM7 BIOS size".to_string())?;
            Some(Bytes::from(arr))
        } else {
            None
        };
        let arm9_bios_data = if let Some(path) = arm9_bios {
            let data = fs::read(path).map_err(|e| format!("Failed to read ARM9 BIOS: {}", e))?;
            let arr: [u8; 4096] = data.try_into().map_err(|_| "Invalid ARM9 BIOS size".to_string())?;
            Some(Bytes::from(arr))
        } else {
            None
        };

        let (renderer_3d_tx, renderer_3d_rx) = soft_renderer_3d::init();
        let renderer_2d = Box::new(Renderer2d::new(Box::new(renderer_3d_rx)));

        let firmware_data = if let Some(path) = firmware {
             fs::read(path).map_err(|e| format!("Failed to read firmware: {}", e))?
        } else {
             firmware::default(Model::Lite).to_vec()
        };
        
        let mut firmware_bbs = BoxedByteSlice::new_zeroed(firmware_data.len());
        firmware_bbs.copy_from_slice(&firmware_data);

        let firmware_flash = Flash::new(
             SaveContents::Existing(firmware_bbs), 
             firmware::id_for_model(Model::Lite),
        ).expect("Failed to create firmware");

        // Load or create save data
        let save_contents = if let Ok(data) = fs::read(save_path) {
            let mut bbs = BoxedByteSlice::new_zeroed(data.len());
            bbs.copy_from_slice(&data);
            SaveContents::Existing(bbs)
        } else {
            // Use 512KB Flash with IR enabled by default. Initialized with 0xFF.
            let mut save_bbs = BoxedByteSlice::new_zeroed(0x80000);
            save_bbs.fill(0xFF);
            SaveContents::Existing(save_bbs)
        };
        
        let spi_device = ds_slot::spi::flash::Flash::new(
            save_contents,
            [0x20, 0x40, 0x13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], // ST M25P40 ID
            true, // Enable IR
        ).expect("Failed to create SPI Flash");
        let spi: ds_slot::spi::Spi = spi_device.into();

        let audio_backend = Box::new(DsAudioBackend::new(audio_subsystem));

        let mut emu_builder = emu::Builder::new(
            firmware_flash,
            Some(Box::new(rom_boxed_byte_slice)),
            spi,
            audio_backend,
            None,
            Box::new(RtcBackend::new(0)),
            renderer_2d,
            Box::new(renderer_3d_tx),
            None,
        );
        
        if let Some(data) = arm7_bios_data {
            emu_builder.arm7_bios = Some(Box::new(data));
        }
        if let Some(data) = arm9_bios_data {
            emu_builder.arm9_bios = Some(Box::new(data));
        }
        
        emu_builder.direct_boot = true;

        let emu = emu_builder.build(Interpreter).map_err(|_| "Failed to build emulator (BuildError)".to_string())?;
        
        Ok(DsEmulator { emu })
    }
    
    pub fn step(&mut self) {
        self.emu.run();
    }

    pub fn save_dirty(&self) -> bool {
        self.emu.ds_slot.spi.contents_dirty()
    }

    pub fn save_data(&self) -> &[u8] {
        self.emu.ds_slot.spi.contents()
    }

    pub fn mark_save_flushed(&mut self) {
        self.emu.ds_slot.spi.mark_contents_flushed();
    }

    pub fn set_touch_pos(&mut self, x: u16, y: u16) {
        // DS touch screen coordinates are 0-4095 in TSC space.
        // Logical pixels are 0-255 (x) and 0-191 (y).
        // Scaling:
        let touch_x = ((x as u32 * 4096) / 256) as u16;
        let touch_y = ((y as u32 * 4096) / 192) as u16;
        self.emu.set_touch_pos([touch_x, touch_y]);
    }

    pub fn end_touch(&mut self) {
        self.emu.end_touch();
    }
}