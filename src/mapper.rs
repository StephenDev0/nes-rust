pub struct MapperFactory;
use rom::Mirrorings;
use rom::RomHeader;
use register::Register;
use save_state::MapperState;

impl MapperFactory {
	pub fn create(header: &RomHeader) -> Option<Box<dyn Mapper>> {
		match header.mapper_num() {
			0 => Some(Box::new(NRomMapper::new(header))),
			1 => Some(Box::new(MMC1Mapper::new(header))),
			2 => Some(Box::new(UNRomMapper::new(header))),
			3 => Some(Box::new(CNRomMapper::new())),
			4 => Some(Box::new(MMC3Mapper::new(header))),
            69 => Some(Box::new(SunsoftMapper::new(header))),
			_ => None
		}
	}
}

pub trait Mapper {
	// Maps 0x8000 - 0xFFFF to the program rom address
	fn map(&self, address: u32) -> u32;

	// Maps 0x0000 - 0x1FFF to the character rom address
	fn map_for_chr_rom(&self, address: u32) -> u32;

	// Writes control register inside in general
	fn store(&mut self, address: u32, value: u8);

	fn has_mirroring_type(&self) -> bool;

	fn mirroring_type(&self) -> Mirrorings;

	// @TODO: MMC3Mapper specific. Should this method be here?
	fn drive_irq_counter(&mut self) -> bool;

	// Save mapper state
	fn save_state(&self) -> MapperState;

	// Load mapper state
	fn load_state(&mut self, state: &MapperState);
}

pub struct NRomMapper {
	program_bank_num: u8
}

impl NRomMapper {
	fn new(header: &RomHeader) -> Self {
		NRomMapper {
			program_bank_num: header.prg_rom_bank_num()
		}
	}
}

impl Mapper for NRomMapper {
	/**
	 * if program_bank_num == 1:
	 * 	0x8000 - 0xBFFF: 0x0000 - 0x3FFF
	 *	0xC000 - 0xFFFF: 0x0000 - 0x3FFF
	 * else:
	 * 	0x8000 - 0xFFFF: 0x0000 - 0x7FFF
	 */
	fn map(&self, mut address: u32) -> u32 {
		if self.program_bank_num == 1 && address >= 0xC000 {
			address -= 0x4000;
		}
		address - 0x8000
	}

	/**
	 * 0x0000 - 0x1FFF: 0x0000 - 0x1FFF
	 */
	fn map_for_chr_rom(&self, address: u32) -> u32 {
		address
	}

	/**
	 * Nothing to do
	 */
	fn store(&mut self, _address: u32, _value: u8) {
		// throw exception?
	}

	fn has_mirroring_type(&self) -> bool {
		false
	}

	fn mirroring_type(&self) -> Mirrorings {
		Mirrorings::SingleScreen // dummy
	}

	fn drive_irq_counter(&mut self) -> bool {
		false
	}

	fn save_state(&self) -> MapperState {
		MapperState::NRom {
			program_bank_num: self.program_bank_num,
		}
	}

	fn load_state(&mut self, state: &MapperState) {
		if let MapperState::NRom { program_bank_num } = state {
			self.program_bank_num = *program_bank_num;
		}
	}
}

pub struct MMC1Mapper {
	program_bank_num: u8,
	control_register: Register<u8>,
	chr_bank0_register: Register<u8>,
	chr_bank1_register: Register<u8>,
	prg_bank_register: Register<u8>,
	latch: Register<u8>,
	register_write_count: u32
}

impl MMC1Mapper {
	fn new(header: &RomHeader) -> Self {
		let mut control_register = Register::<u8>::new();
		control_register.store(0x0C);
		MMC1Mapper {
			program_bank_num: header.prg_rom_bank_num(),
			control_register: control_register,
			chr_bank0_register: Register::<u8>::new(),
			chr_bank1_register: Register::<u8>::new(),
			prg_bank_register: Register::<u8>::new(),
			latch: Register::<u8>::new(),
			register_write_count: 0
		}
	}
}

impl Mapper for MMC1Mapper {
	fn map(&self, address: u32) -> u32 {
		let bank: u32;
		let mut offset = address & 0x3FFF;
		let bank_num = self.prg_bank_register.load() as u32 & 0x0F;

		match self.control_register.load_bits(2, 2) {
			0 | 1 => {
				// switch 32KB at 0x8000, ignoring low bit of bank number
				// TODO: Fix me
				offset = offset | (address & 0x4000);
				bank = bank_num & 0x0E;
			},
			2 => {
				// fix first bank at 0x8000 and switch 16KB bank at 0xC000
				bank = match address < 0xC000 {
					true => 0,
					false => bank_num
				};
			},
			_ /*3*/ => {
				// fix last bank at 0xC000 and switch 16KB bank at 0x8000
				bank = match address >= 0xC000 {
					true => self.program_bank_num as u32 - 1,
					false => bank_num
				};
			}
		};
		bank * 0x4000 + offset
	}

	fn map_for_chr_rom(&self, address: u32) -> u32 {
		let bank: u32;
		let mut offset = address & 0x0FFF;
		if self.control_register.load_bit(4) == 0 {
			// switch 8KB at a time
			bank = self.chr_bank0_register.load() as u32 & 0x1E;
			offset = offset | (address & 0x1000);
		} else {
			// switch two separate 4KB banks
			bank = match address < 0x1000 {
				true => self.chr_bank0_register.load(),
				false => self.chr_bank1_register.load()
			} as u32 & 0x1f;
		}
		bank * 0x1000 + offset
	}

	fn store(&mut self, address: u32, value: u8) {
		if (value & 0x80) != 0 {
			self.register_write_count = 0;
			self.latch.clear();
			if (address & 0x6000) == 0 {
				self.control_register.store_bits(2, 2, 3);
			}
		} else {
			self.latch.store(((value & 1) << 4) | (self.latch.load() >> 1));
			self.register_write_count += 1;

			if self.register_write_count >= 5 {
				let val = self.latch.load();
				match address & 0x6000 {
					0x0000 => self.control_register.store(val),
					0x2000 => self.chr_bank0_register.store(val),
					0x4000 => self.chr_bank1_register.store(val),
					_ /*0x6000*/ => self.prg_bank_register.store(val)
				};
				self.register_write_count = 0;
				self.latch.clear();
			}
		}
	}

	fn has_mirroring_type(&self) -> bool {
		true
	}

	fn mirroring_type(&self) -> Mirrorings {
		match self.control_register.load_bits(0, 2) {
			0 | 1 => Mirrorings::SingleScreen,
			2 => Mirrorings::Vertical,
			_ /*3*/ => Mirrorings::Horizontal
		}
	}

	fn drive_irq_counter(&mut self) -> bool {
		false
	}

	fn save_state(&self) -> MapperState {
		MapperState::MMC1 {
			program_bank_num: self.program_bank_num,
			control_register: self.control_register.get_data(),
			chr_bank0_register: self.chr_bank0_register.get_data(),
			chr_bank1_register: self.chr_bank1_register.get_data(),
			prg_bank_register: self.prg_bank_register.get_data(),
			latch: self.latch.get_data(),
			register_write_count: self.register_write_count,
		}
	}

	fn load_state(&mut self, state: &MapperState) {
		if let MapperState::MMC1 {
			program_bank_num,
			control_register,
			chr_bank0_register,
			chr_bank1_register,
			prg_bank_register,
			latch,
			register_write_count,
		} = state {
			self.program_bank_num = *program_bank_num;
			self.control_register.set_data(*control_register);
			self.chr_bank0_register.set_data(*chr_bank0_register);
			self.chr_bank1_register.set_data(*chr_bank1_register);
			self.prg_bank_register.set_data(*prg_bank_register);
			self.latch.set_data(*latch);
			self.register_write_count = *register_write_count;
		}
	}
}

struct UNRomMapper {
	program_bank_num: u8,
	register: Register<u8>
}

impl UNRomMapper {
	fn new(header: &RomHeader) -> Self {
		UNRomMapper {
			program_bank_num: header.prg_rom_bank_num(),
			register: Register::<u8>::new()
		}
	}
}

impl Mapper for UNRomMapper {
	fn map(&self, address: u32) -> u32 {
		let bank = match address < 0xC000 {
			true => self.register.load(),
			false => self.program_bank_num - 1
		} as u32;
		let offset = address & 0x3FFF;
		0x4000 * bank + offset
	}

	fn map_for_chr_rom(&self, address: u32) -> u32 {
		address
	}

	fn store(&mut self, _address: u32, value: u8) {
		self.register.store(value & 0xF);
	}

	fn has_mirroring_type(&self) -> bool {
		false
	}

	fn mirroring_type(&self) -> Mirrorings {
		Mirrorings::SingleScreen // dummy
	}

	fn drive_irq_counter(&mut self) -> bool {
		false
	}

	fn save_state(&self) -> MapperState {
		MapperState::UNRom {
			program_bank_num: self.program_bank_num,
			register: self.register.get_data(),
		}
	}

	fn load_state(&mut self, state: &MapperState) {
		if let MapperState::UNRom { program_bank_num, register } = state {
			self.program_bank_num = *program_bank_num;
			self.register.set_data(*register);
		}
	}
}

struct CNRomMapper {
	register: Register<u8>
}

impl CNRomMapper {
	fn new() -> Self {
		CNRomMapper {
			register: Register::<u8>::new()
		}
	}
}

impl Mapper for CNRomMapper {
	fn map(&self, address: u32) -> u32 {
		address - 0x8000
	}

	fn map_for_chr_rom(&self, address: u32) -> u32 {
		self.register.load() as u32 * 0x2000 + (address & 0x1FFF)
	}

	fn store(&mut self, _address: u32, value: u8) {
		self.register.store(value & 0xF);
	}

	fn has_mirroring_type(&self) -> bool {
		false
	}

	fn mirroring_type(&self) -> Mirrorings {
		Mirrorings::SingleScreen // dummy
	}

	fn drive_irq_counter(&mut self) -> bool {
		false
	}

	fn save_state(&self) -> MapperState {
		MapperState::CNRom {
			register: self.register.get_data(),
		}
	}

	fn load_state(&mut self, state: &MapperState) {
		if let MapperState::CNRom { register } = state {
			self.register.set_data(*register);
		}
	}
}

struct MMC3Mapper {
	program_bank_num: u8,
	character_bank_num: u8,
	register0: Register<u8>,
	register1: Register<u8>,
	register2: Register<u8>,
	register3: Register<u8>,
	register4: Register<u8>,
	register5: Register<u8>,
	register6: Register<u8>,
	register7: Register<u8>,
	program_register0: Register<u8>,
	program_register1: Register<u8>,
	character_register0: Register<u8>,
	character_register1: Register<u8>,
	character_register2: Register<u8>,
	character_register3: Register<u8>,
	character_register4: Register<u8>,
	character_register5: Register<u8>,
	irq_counter: u8,
	irq_counter_reload: bool,
	irq_enabled: bool
}

impl MMC3Mapper {
	fn new(header: &RomHeader) -> Self {
		MMC3Mapper {
			program_bank_num: header.prg_rom_bank_num(),
			character_bank_num: header.chr_rom_bank_num(),
			register0: Register::<u8>::new(),
			register1: Register::<u8>::new(),
			register2: Register::<u8>::new(),
			register3: Register::<u8>::new(),
			register4: Register::<u8>::new(),
			register5: Register::<u8>::new(),
			register6: Register::<u8>::new(),
			register7: Register::<u8>::new(),
			program_register0: Register::<u8>::new(),
			program_register1: Register::<u8>::new(),
			character_register0: Register::<u8>::new(),
			character_register1: Register::<u8>::new(),
			character_register2: Register::<u8>::new(),
			character_register3: Register::<u8>::new(),
			character_register4: Register::<u8>::new(),
			character_register5: Register::<u8>::new(),
			irq_counter: 0,
			irq_counter_reload: false,
			irq_enabled: true
		}
	}
}

impl Mapper for MMC3Mapper {
	fn map(&self, address: u32) -> u32 {
		let bank = match address {
			0x8000..=0x9FFF => match self.register0.is_bit_set(6) {
				true => self.program_bank_num * 2 - 2,
				false => self.program_register0.load()
			},
			0xA000..=0xBFFF => self.program_register1.load(),
			0xC000..=0xDFFF => match self.register0.is_bit_set(6) {
				true => self.program_register0.load(),
				false => self.program_bank_num * 2 - 2
			},
			_ => self.program_bank_num * 2 - 1
		};
		// I couldn't in the spec but it seems that
		// we need to wrap 2k bank with 4k program_bank_num
		((bank as u32) % ((self.program_bank_num as u32) * 2)) * 0x2000 + (address & 0x1FFF)
	}

	fn map_for_chr_rom(&self, address: u32) -> u32 {
		let bank = match self.register0.is_bit_set(7) {
			true => match address & 0x1FFF {
				0x0000..=0x03FF => self.character_register2.load(),
				0x0400..=0x07FF => self.character_register3.load(),
				0x0800..=0x0BFF => self.character_register4.load(),
				0x0C00..=0x0FFF => self.character_register5.load(),
				0x1000..=0x13FF => self.character_register0.load() & 0xFE,
				0x1400..=0x17FF => self.character_register0.load() | 1,
				0x1800..=0x1BFF => self.character_register1.load() & 0xFE,
				_ => self.character_register1.load() | 1
			},
			false => match address & 0x1FFF {
				0x0000..=0x03FF => self.character_register0.load() & 0xFE,
				0x0400..=0x07FF => self.character_register0.load() | 1,
				0x0800..=0x0BFF => self.character_register1.load() & 0xFE,
				0x0C00..=0x0FFF => self.character_register1.load() | 1,
				0x1000..=0x13FF => self.character_register2.load(),
				0x1400..=0x17FF => self.character_register3.load(),
				0x1800..=0x1BFF => self.character_register4.load(),
				_ => self.character_register5.load()
			}
		};
		// I couldn't in the spec but it seems that
		// we need to wrap 0.4k bank with 4k character_bank_num
		((bank as u32) % ((self.character_bank_num as u32) * 8)) * 0x400 + (address & 0x3FF)
	}

	fn store(&mut self, address: u32, value: u8) {
		match address {
			0x8000..=0x9FFF => match (address & 1) == 0 {
				true => self.register0.store(value),
				false => {
					self.register1.store(value);
					match self.register0.load_bits(0, 3) {
						0 => self.character_register0.store(value & 0xFE),
						1 => self.character_register1.store(value & 0xFE),
						2 => self.character_register2.store(value),
						3 => self.character_register3.store(value),
						4 => self.character_register4.store(value),
						5 => self.character_register5.store(value),
						6 => self.program_register0.store(value & 0x3F),
						_ => self.program_register1.store(value & 0x3F)
					};
				}
			},
			0xA000..=0xBFFF => match (address & 1) == 0 {
				true => self.register2.store(value),
				false => self.register3.store(value)
			},
			0xC000..=0xDFFF => {
				match (address & 1) == 0 {
					true => self.register4.store(value),
					false => self.register5.store(value)
				};
				self.irq_counter_reload = true;
			},
			_ => match (address & 1) == 0 {
				true => {
					self.register6.store(value);
					self.irq_enabled = false;
				},
				false => {
					self.register7.store(value);
					self.irq_enabled = true;
				}
			}
		};
	}

	fn has_mirroring_type(&self) -> bool {
		true
	}

	fn mirroring_type(&self) -> Mirrorings {
		match self.register2.is_bit_set(0) {
			true => Mirrorings::Horizontal,
			false => Mirrorings::Vertical
		}
	}

	fn drive_irq_counter(&mut self) -> bool {
		match self.irq_counter_reload {
			true => {
				self.irq_counter = self.register4.load();
				self.irq_counter_reload = false;
				false
			},
			false => match self.irq_enabled {
				true => match self.irq_counter > 0 {
					true => {
						self.irq_counter -= 1;
						match self.irq_counter == 0 {
							true => {
								self.irq_counter_reload = true;
								true
							}
							false => false
						}
					},
					false => false
				},
				false => false
			}
		}
	}

	fn save_state(&self) -> MapperState {
		MapperState::MMC3 {
			program_bank_num: self.program_bank_num,
			character_bank_num: self.character_bank_num,
			register0: self.register0.get_data(),
			register1: self.register1.get_data(),
			register2: self.register2.get_data(),
			register3: self.register3.get_data(),
			register4: self.register4.get_data(),
			register5: self.register5.get_data(),
			register6: self.register6.get_data(),
			register7: self.register7.get_data(),
			program_register0: self.program_register0.get_data(),
			program_register1: self.program_register1.get_data(),
			character_register0: self.character_register0.get_data(),
			character_register1: self.character_register1.get_data(),
			character_register2: self.character_register2.get_data(),
			character_register3: self.character_register3.get_data(),
			character_register4: self.character_register4.get_data(),
			character_register5: self.character_register5.get_data(),
			irq_counter: self.irq_counter,
			irq_counter_reload: self.irq_counter_reload,
			irq_enabled: self.irq_enabled,
		}
	}

	fn load_state(&mut self, state: &MapperState) {
		if let MapperState::MMC3 {
			program_bank_num,
			character_bank_num,
			register0,
			register1,
			register2,
			register3,
			register4,
			register5,
			register6,
			register7,
			program_register0,
			program_register1,
			character_register0,
			character_register1,
			character_register2,
			character_register3,
			character_register4,
			character_register5,
			irq_counter,
			irq_counter_reload,
			irq_enabled,
		} = state {
			self.program_bank_num = *program_bank_num;
			self.character_bank_num = *character_bank_num;
			self.register0.set_data(*register0);
			self.register1.set_data(*register1);
			self.register2.set_data(*register2);
			self.register3.set_data(*register3);
			self.register4.set_data(*register4);
			self.register5.set_data(*register5);
			self.register6.set_data(*register6);
			self.register7.set_data(*register7);
			self.program_register0.set_data(*program_register0);
			self.program_register1.set_data(*program_register1);
			self.character_register0.set_data(*character_register0);
			self.character_register1.set_data(*character_register1);
			self.character_register2.set_data(*character_register2);
			self.character_register3.set_data(*character_register3);
			self.character_register4.set_data(*character_register4);
			self.character_register5.set_data(*character_register5);
			self.irq_counter = *irq_counter;
			self.irq_counter_reload = *irq_counter_reload;
			self.irq_enabled = *irq_enabled;
		}
	}
}


struct SunsoftMapper {
    command_register: Register<u8>,
    parameter_register: Register<u8>,
    chr_banks: [Register<u8>; 8],
    prg_banks: [Register<u8>; 4],
    irq_enabled: bool,
    irq_counter_enabled: bool,
    irq_counter: u16,
    mirroring: u8,
    prg_bank_mask: u32,
    chr_bank_mask: u32,
}

impl SunsoftMapper {
    fn new(header: &RomHeader) -> Self {
        let prg_bank_num = header.prg_rom_bank_num() as u32 * 2; // 8KB banks
        let chr_bank_num = header.chr_rom_bank_num() as u32 * 8; // 1KB banks

        let mut m = SunsoftMapper {
            command_register: Register::<u8>::new(),
            parameter_register: Register::<u8>::new(),
            chr_banks: [
                Register::<u8>::new(), Register::<u8>::new(),
                Register::<u8>::new(), Register::<u8>::new(),
                Register::<u8>::new(), Register::<u8>::new(),
                Register::<u8>::new(), Register::<u8>::new(),
            ],
            prg_banks: [
                Register::<u8>::new(), Register::<u8>::new(),
                Register::<u8>::new(), Register::<u8>::new(),
            ],
            irq_enabled: false,
            irq_counter_enabled: false,
            irq_counter: 0,
            mirroring: 0,
            prg_bank_mask: prg_bank_num.saturating_sub(1),
            chr_bank_mask: chr_bank_num.saturating_sub(1),
        };
        // Defaults
        m.prg_banks[3].store((prg_bank_num - 1) as u8); // Fixed last bank
        m
    }
}

impl Mapper for SunsoftMapper {
    fn map(&self, address: u32) -> u32 {
        let bank_idx = match address {
            0x6000..=0x7FFF => 0,
            0x8000..=0x9FFF => 1,
            0xA000..=0xBFFF => 2,
            0xC000..=0xDFFF => 3,
            0xE000..=0xFFFF => return ((self.prg_bank_mask) * 0x2000) + (address & 0x1FFF),
            _ => 0,
        };
        
        let bank = self.prg_banks[bank_idx].load() as u32 & self.prg_bank_mask;
        bank * 0x2000 + (address & 0x1FFF)
    }

    fn map_for_chr_rom(&self, address: u32) -> u32 {
        let bank_idx = (address / 0x400) as usize;
        if bank_idx < 8 {
            let bank = self.chr_banks[bank_idx].load() as u32 & self.chr_bank_mask;
            bank * 0x400 + (address & 0x3FF)
        } else {
            address
        }
    }

    fn store(&mut self, address: u32, value: u8) {
        match address {
            0x8000..=0x9FFF => {
                self.command_register.store(value & 0x0F);
            }
            0xA000..=0xBFFF => {
                self.parameter_register.store(value);
                let cmd = self.command_register.load();
                match cmd {
                    0..=7 => {
                        self.chr_banks[cmd as usize].store(value);
                    }
                    8..=11 => {
                        self.prg_banks[(cmd - 8) as usize].store(value);
                    }
                    12 => {
                        self.mirroring = value & 0x03;
                    }
                    13 => {
                        self.irq_enabled = (value & 0x80) != 0;
                        self.irq_counter_enabled = (value & 0x01) != 0;
                    }
                    14 => {
                        self.irq_counter = (self.irq_counter & 0xFF00) | (value as u16);
                    }
                    15 => {
                        self.irq_counter = (self.irq_counter & 0x00FF) | ((value as u16) << 8);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn has_mirroring_type(&self) -> bool {
        true
    }

    fn mirroring_type(&self) -> Mirrorings {
        match self.mirroring {
            0 => Mirrorings::Vertical,
            1 => Mirrorings::Horizontal,
            2 => Mirrorings::OneScreenLow, // OneScreen 0
            3 => Mirrorings::OneScreenHigh, // OneScreen 1
            _ => Mirrorings::Vertical,
        }
    }

    fn drive_irq_counter(&mut self) -> bool {
        if self.irq_counter_enabled {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
            if self.irq_counter == 0xFFFF {
                return self.irq_enabled;
            }
        }
        false
    }

    fn save_state(&self) -> MapperState {
        MapperState::Sunsoft {
            command_register: self.command_register.get_data(),
            parameter_register: self.parameter_register.get_data(),
            chr_banks: [
                self.chr_banks[0].get_data(), self.chr_banks[1].get_data(),
                self.chr_banks[2].get_data(), self.chr_banks[3].get_data(),
                self.chr_banks[4].get_data(), self.chr_banks[5].get_data(),
                self.chr_banks[6].get_data(), self.chr_banks[7].get_data(),
            ],
            prg_banks: [
                self.prg_banks[0].get_data(), self.prg_banks[1].get_data(),
                self.prg_banks[2].get_data(), self.prg_banks[3].get_data(),
            ],
            irq_enabled: self.irq_enabled,
            irq_counter_enabled: self.irq_counter_enabled,
            irq_counter: self.irq_counter,
            mirroring: self.mirroring,
        }
    }

    fn load_state(&mut self, state: &MapperState) {
        if let MapperState::Sunsoft {
            command_register,
            parameter_register,
            chr_banks,
            prg_banks,
            irq_enabled,
            irq_counter_enabled,
            irq_counter,
            mirroring,
        } = state {
            self.command_register.set_data(*command_register);
            self.parameter_register.set_data(*parameter_register);
            for (i, val) in chr_banks.iter().enumerate() {
                self.chr_banks[i].set_data(*val);
            }
            for (i, val) in prg_banks.iter().enumerate() {
                self.prg_banks[i].set_data(*val);
            }
            self.irq_enabled = *irq_enabled;
            self.irq_counter_enabled = *irq_counter_enabled;
            self.irq_counter = *irq_counter;
            self.mirroring = *mirroring;
        }
    }
}

#[cfg(test)]
mod tests_nrom_mapper {
	use super::*;

	#[test]
	fn initialize() {
		NRomMapper{program_bank_num: 1};
	}

	#[test]
	fn map_with_program_bank_num_1() {
		let m = NRomMapper{program_bank_num: 1};
		assert_eq!(0x0000, m.map(0x8000));
		assert_eq!(0x3FFF, m.map(0xBFFF));
		assert_eq!(0x0000, m.map(0xC000));
		assert_eq!(0x3FFF, m.map(0xFFFF));
	}

	#[test]
	fn map_with_program_bank_num_2() {
		let m = NRomMapper{program_bank_num: 2};
		assert_eq!(0x0000, m.map(0x8000));
		assert_eq!(0x3FFF, m.map(0xBFFF));
		assert_eq!(0x4000, m.map(0xC000));
		assert_eq!(0x7FFF, m.map(0xFFFF));
	}

	#[test]
	fn map_for_chr_rom() {
		let m = NRomMapper{program_bank_num: 1};
		assert_eq!(0x0000, m.map_for_chr_rom(0x0000));
		assert_eq!(0x1FFF, m.map_for_chr_rom(0x1FFF));
	}
}
