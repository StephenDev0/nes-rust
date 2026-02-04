use serde::{Serialize, Deserialize};

/// Save state version for compatibility checking
pub const SAVE_STATE_VERSION: u32 = 1;

/// Complete NES save state
#[derive(Serialize, Deserialize)]
pub struct SaveState {
    pub version: u32,
    pub cpu: CpuState,
    pub ppu: PpuState,
    pub apu: ApuState,
    pub joypad1: JoypadState,
    pub joypad2: JoypadState,
    pub mapper: MapperState,
}

impl SaveState {
    pub fn new() -> Self {
        SaveState {
            version: SAVE_STATE_VERSION,
            cpu: CpuState::new(),
            ppu: PpuState::new(),
            apu: ApuState::new(),
            joypad1: JoypadState::new(),
            joypad2: JoypadState::new(),
            mapper: MapperState::None,
        }
    }
}

/// CPU state
#[derive(Serialize, Deserialize)]
pub struct CpuState {
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub ram: Vec<u8>,
    pub stall_cycles: u16,
}

impl CpuState {
    pub fn new() -> Self {
        CpuState {
            pc: 0,
            sp: 0,
            a: 0,
            x: 0,
            y: 0,
            p: 0,
            ram: vec![0; 64 * 1024],
            stall_cycles: 0,
        }
    }
}

/// PPU state
#[derive(Serialize, Deserialize)]
pub struct PpuState {
    pub frame: u32,
    pub cycle: u16,
    pub scanline: u16,
    pub suppress_vblank: bool,
    pub register_first_store: bool,
    pub fine_x_scroll: u8,
    pub name_table_latch: u8,
    pub name_table: u8,
    pub pattern_table_low_latch: u8,
    pub pattern_table_high_latch: u8,
    pub pattern_table_low: u16,
    pub pattern_table_high: u16,
    pub attribute_table_low_latch: u8,
    pub attribute_table_high_latch: u8,
    pub attribute_table_low: u16,
    pub attribute_table_high: u16,
    pub current_vram_address: u16,
    pub temporal_vram_address: u16,
    pub vram_read_buffer: u8,
    pub vram: Vec<u8>,
    pub primary_oam: Vec<u8>,
    pub secondary_oam: Vec<u8>,
    pub oamaddr: u8,
    pub ppuctrl: u8,
    pub ppumask: u8,
    pub ppustatus: u8,
    pub data_bus: u8,
    pub nmi_interrupted: bool,
    pub irq_interrupted: bool,
}

impl PpuState {
    pub fn new() -> Self {
        PpuState {
            frame: 0,
            cycle: 0,
            scanline: 0,
            suppress_vblank: false,
            register_first_store: true,
            fine_x_scroll: 0,
            name_table_latch: 0,
            name_table: 0,
            pattern_table_low_latch: 0,
            pattern_table_high_latch: 0,
            pattern_table_low: 0,
            pattern_table_high: 0,
            attribute_table_low_latch: 0,
            attribute_table_high_latch: 0,
            attribute_table_low: 0,
            attribute_table_high: 0,
            current_vram_address: 0,
            temporal_vram_address: 0,
            vram_read_buffer: 0,
            vram: vec![0; 16 * 1024],
            primary_oam: vec![0; 256],
            secondary_oam: vec![0; 32],
            oamaddr: 0,
            ppuctrl: 0,
            ppumask: 0,
            ppustatus: 0,
            data_bus: 0,
            nmi_interrupted: false,
            irq_interrupted: false,
        }
    }
}

/// APU state
#[derive(Serialize, Deserialize)]
pub struct ApuState {
    pub cycle: u32,
    pub step: u16,
    pub pulse1: ApuPulseState,
    pub pulse2: ApuPulseState,
    pub triangle: ApuTriangleState,
    pub noise: ApuNoiseState,
    pub dmc: ApuDmcState,
    pub status: u8,
    pub frame: u8,
    pub frame_irq_active: bool,
    pub dmc_irq_active: bool,
}

impl ApuState {
    pub fn new() -> Self {
        ApuState {
            cycle: 0,
            step: 0,
            pulse1: ApuPulseState::new(),
            pulse2: ApuPulseState::new(),
            triangle: ApuTriangleState::new(),
            noise: ApuNoiseState::new(),
            dmc: ApuDmcState::new(),
            status: 0,
            frame: 0,
            frame_irq_active: false,
            dmc_irq_active: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ApuPulseState {
    pub register0: u8,
    pub register1: u8,
    pub register2: u8,
    pub register3: u8,
    pub enabled: bool,
    pub timer_counter: u16,
    pub timer_period: u16,
    pub timer_sequence: u8,
    pub envelope_start_flag: bool,
    pub envelope_counter: u8,
    pub envelope_decay_level_counter: u8,
    pub length_counter: u8,
    pub sweep_reload_flag: bool,
    pub sweep_counter: u8,
}

impl ApuPulseState {
    pub fn new() -> Self {
        ApuPulseState {
            register0: 0,
            register1: 0,
            register2: 0,
            register3: 0,
            enabled: false,
            timer_counter: 0,
            timer_period: 0,
            timer_sequence: 0,
            envelope_start_flag: true,
            envelope_counter: 0,
            envelope_decay_level_counter: 0,
            length_counter: 0,
            sweep_reload_flag: false,
            sweep_counter: 0,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ApuTriangleState {
    pub register0: u8,
    pub register2: u8,
    pub register3: u8,
    pub enabled: bool,
    pub timer_counter: u16,
    pub timer_sequence: u8,
    pub length_counter: u8,
    pub linear_reload_flag: bool,
    pub linear_counter: u8,
}

impl ApuTriangleState {
    pub fn new() -> Self {
        ApuTriangleState {
            register0: 0,
            register2: 0,
            register3: 0,
            enabled: false,
            timer_counter: 0,
            timer_sequence: 0,
            length_counter: 0,
            linear_reload_flag: false,
            linear_counter: 0,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ApuNoiseState {
    pub register0: u8,
    pub register2: u8,
    pub register3: u8,
    pub enabled: bool,
    pub timer_counter: u16,
    pub timer_period: u16,
    pub envelope_start_flag: bool,
    pub envelope_counter: u8,
    pub envelope_decay_level_counter: u8,
    pub length_counter: u8,
    pub shift_register: u16,
}

impl ApuNoiseState {
    pub fn new() -> Self {
        ApuNoiseState {
            register0: 0,
            register2: 0,
            register3: 0,
            enabled: false,
            timer_counter: 0,
            timer_period: 0,
            envelope_start_flag: false,
            envelope_counter: 0,
            envelope_decay_level_counter: 0,
            length_counter: 0,
            shift_register: 1,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ApuDmcState {
    pub register0: u8,
    pub register1: u8,
    pub register2: u8,
    pub register3: u8,
    pub enabled: bool,
    pub timer_period: u16,
    pub timer_counter: u16,
    pub delta_counter: u8,
    pub address_counter: u16,
    pub remaining_bytes_counter: u16,
    pub sample_buffer: u8,
    pub sample_buffer_is_empty: bool,
    pub shift_register: u8,
    pub remaining_bits_counter: u8,
    pub silence_flag: bool,
}

impl ApuDmcState {
    pub fn new() -> Self {
        ApuDmcState {
            register0: 0,
            register1: 0,
            register2: 0,
            register3: 0,
            enabled: false,
            timer_period: 0,
            timer_counter: 0,
            delta_counter: 0,
            address_counter: 0,
            remaining_bytes_counter: 0,
            sample_buffer: 0,
            sample_buffer_is_empty: true,
            shift_register: 0,
            remaining_bits_counter: 0,
            silence_flag: true,
        }
    }
}

/// Joypad state
#[derive(Serialize, Deserialize)]
pub struct JoypadState {
    pub register: u8,
    pub latch: u8,
    pub current_button: u8,
    pub buttons: [bool; 8],
}

impl JoypadState {
    pub fn new() -> Self {
        JoypadState {
            register: 0,
            latch: 0,
            current_button: 0,
            buttons: [false; 8],
        }
    }
}

/// Mapper state - each mapper type has different state
#[derive(Serialize, Deserialize)]
pub enum MapperState {
    None,
    NRom {
        program_bank_num: u8,
    },
    MMC1 {
        program_bank_num: u8,
        control_register: u8,
        chr_bank0_register: u8,
        chr_bank1_register: u8,
        prg_bank_register: u8,
        latch: u8,
        register_write_count: u32,
    },
    UNRom {
        program_bank_num: u8,
        register: u8,
    },
    CNRom {
        register: u8,
    },
    MMC3 {
        program_bank_num: u8,
        character_bank_num: u8,
        register0: u8,
        register1: u8,
        register2: u8,
        register3: u8,
        register4: u8,
        register5: u8,
        register6: u8,
        register7: u8,
        program_register0: u8,
        program_register1: u8,
        character_register0: u8,
        character_register1: u8,
        character_register2: u8,
        character_register3: u8,
        character_register4: u8,
        character_register5: u8,
        irq_counter: u8,
        irq_counter_reload: bool,
        irq_enabled: bool,
    },
    Sunsoft {
        command_register: u8,
        parameter_register: u8,
        chr_banks: [u8; 8],
        prg_banks: [u8; 4],
        irq_enabled: bool,
        irq_counter_enabled: bool,
        irq_counter: u16,
        mirroring: u8,
    },
}

/// Serialize a save state to bytes
pub fn serialize(state: &SaveState) -> Result<Vec<u8>, String> {
    bincode::serialize(state).map_err(|e| format!("Serialization failed: {}", e))
}

/// Deserialize bytes to a save state
pub fn deserialize(data: &[u8]) -> Result<SaveState, String> {
    let state: SaveState = bincode::deserialize(data)
        .map_err(|e| format!("Deserialization failed: {}", e))?;

    if state.version != SAVE_STATE_VERSION {
        return Err(format!(
            "Save state version mismatch: expected {}, got {}",
            SAVE_STATE_VERSION, state.version
        ));
    }

    Ok(state)
}
