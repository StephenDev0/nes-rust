extern crate nes_rust;
extern crate sdl2;

mod sdl2_input;
mod sdl2_display;
mod sdl2_audio;

use std::fs::File;
use std::io::Read;
use std::time::Duration;
use std::env;
use std::path::Path;

use nes_rust::Nes;
use nes_rust::rom::Rom;
use nes_rust::ds::emulator::DsEmulator;

use sdl2_input::Sdl2Input;
use sdl2_display::Sdl2Display;
use sdl2_audio::Sdl2Audio;

fn run_ds(rom_path: &str, sdl: sdl2::Sdl) -> std::io::Result<()> {
    let mut emu = DsEmulator::new(Path::new(rom_path), None, None, None)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let video_subsystem = sdl.video().unwrap();
    let window = video_subsystem.window("nes-rust-ds", 256, 384)
        .position_centered().build().unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_streaming(
        sdl2::pixels::PixelFormatEnum::RGBA32, 256, 384).unwrap();

    let mut event_pump = sdl.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit {..} => break 'running,
                _ => {}
            }
        }
        
        emu.step();
        
        let fb = emu.emu.gpu.renderer_2d().framebuffer();
        
        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            // Assume fb is a list of screens/scanlines.
            // If fb iterates as &[u32; 49152], it means we have screens.
            for (screen_idx, screen_pixels) in fb.iter().enumerate() {
                for (i, &px) in screen_pixels.iter().enumerate() {
                    let x = i % 256;
                    let y_local = i / 256;
                    // Assuming screen 0 is top (0-191), screen 1 is bottom (192-383)
                    let y_global = screen_idx * 192 + y_local;
                    
                    if y_global >= 384 { continue; }

                    let offset = y_global * pitch;
                    let buf_offset = offset + x * 4;
                    
                    let r = ((px & 0x3F) << 2) as u8;
                    let g = (((px >> 6) & 0x3F) << 2) as u8;
                    let b = (((px >> 12) & 0x3F) << 2) as u8;
                    let a = 255;
                    
                    buffer[buf_offset] = r;
                    buffer[buf_offset + 1] = g;
                    buffer[buf_offset + 2] = b;
                    buffer[buf_offset + 3] = a;
                }
            }
        }).unwrap();
        
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
    
    Ok(())
}

fn main() -> std::io::Result<()> {
	let args: Vec<String> = env::args().collect();

	if args.len() < 2 {
		return Ok(());
	}

	let filename = &args[1];
    let sdl = sdl2::init().unwrap();

    if filename.ends_with(".nds") {
        return run_ds(filename, sdl);
    }

	let mut file = File::open(filename)?;
	let mut contents = vec![];
	file.read_to_end(&mut contents)?;
	let rom = match Rom::new(contents) {
        Some(r) => r,
        None => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid ROM")),
    };
	assert_eq!(rom.valid(), true);

	let event_pump = sdl.event_pump().unwrap();
	let audio_subsystem = sdl.audio().unwrap();
	let input = Box::new(Sdl2Input::new(event_pump));
	let display = Box::new(Sdl2Display::new(sdl));
	let audio = Box::new(Sdl2Audio::new(audio_subsystem));
	let mut nes = Nes::new(input, display, audio);
	nes.set_rom(rom);

	nes.bootup();
	loop {
		nes.step_frame();
		if !nes.is_power_on() {
			break;
		}
		std::thread::sleep(Duration::from_millis(1));
	}
	Ok(())
}
