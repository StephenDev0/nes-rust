extern crate sdl2;
#[macro_use]
extern crate lazy_static;
extern crate serde;
extern crate bincode;

pub mod register;
pub mod cpu;
pub mod ppu;
pub mod apu;
pub mod rom;
pub mod memory;
pub mod mapper;
pub mod button;
pub mod joypad;
pub mod input;
pub mod audio;
pub mod display;
pub mod ds;
pub mod default_input;
pub mod default_audio;
pub mod default_display;
pub mod sdl_backend;
pub mod ffi;
pub mod save_state;

use cpu::Cpu;
use rom::Rom;
use button::Button;
use input::Input;
use display::Display;
use audio::Audio;
use save_state::SaveState;

pub struct Nes {
	cpu: Cpu
}

impl Nes {
	/// Save the current emulator state
	pub fn save_state(&self) -> SaveState {
		self.cpu.save_state()
	}

	/// Load a previously saved state
	pub fn load_state(&mut self, state: &SaveState) {
		self.cpu.load_state(state);
	}

	/// Save state to bytes
	pub fn save_state_to_bytes(&self) -> Result<Vec<u8>, String> {
		let state = self.save_state();
		save_state::serialize(&state)
	}

	/// Load state from bytes
	pub fn load_state_from_bytes(&mut self, data: &[u8]) -> Result<(), String> {
		let state = save_state::deserialize(data)?;
		self.load_state(&state);
		Ok(())
	}
}

impl Nes {
	pub fn new(input: Box<dyn Input>, display: Box<dyn Display>,
		audio: Box<dyn Audio>) -> Self {
		Nes {
			cpu: Cpu::new(
				input,
				display,
				audio
			)
		}
	}

	pub fn set_rom(&mut self, rom: Rom) {
		self.cpu.set_rom(rom);
	}

	pub fn bootup(&mut self) {
		self.cpu.bootup();
	}

	pub fn reset(&mut self) {
		self.cpu.reset();
	}

	pub fn step(&mut self) {
		self.cpu.step();
	}

	pub fn step_frame(&mut self) {
		self.cpu.step_frame();
	}

	pub fn copy_pixels(&self, pixels: &mut [u8]) {
		self.cpu.get_ppu().get_display().copy_to_rgba_pixels(pixels);
	}

	pub fn copy_sample_buffer(&mut self, buffer: &mut [f32]) {
		self.cpu.get_mut_apu().get_mut_audio().copy_sample_buffer(buffer);
	}

	pub fn press_button(&mut self, button: Button) {
		self.cpu.get_mut_input().press(button);
	}

	pub fn release_button(&mut self, button: Button) {
		self.cpu.get_mut_input().release(button);
	}

	pub fn is_power_on(&self) -> bool {
		self.cpu.is_power_on()
	}
}