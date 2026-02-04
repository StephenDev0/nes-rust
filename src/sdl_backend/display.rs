use sdl2::render::{Canvas, Texture, TextureAccess};
use sdl2::Sdl;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::TextureCreator;
use sdl2::video::Window;
use sdl2::video::WindowContext;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::display::Display;
use crate::display::SCREEN_WIDTH;
use crate::display::SCREEN_HEIGHT;
use crate::display::PIXEL_BYTES;
use crate::display::PIXELS_CAPACITY;

pub struct SharedDisplay {
	pixels: [u8; PIXELS_CAPACITY],
    shared_buffer: Arc<Mutex<[u8; PIXELS_CAPACITY]>>,
    frame_ready: Arc<AtomicBool>,
}

impl SharedDisplay {
	pub fn new(shared_buffer: Arc<Mutex<[u8; PIXELS_CAPACITY]>>, frame_ready: Arc<AtomicBool>) -> Self {
		SharedDisplay {
			pixels: [0; PIXELS_CAPACITY],
            shared_buffer,
            frame_ready,
		}
	}
}

impl Display for SharedDisplay {
	#[inline(always)]
	fn render_pixel(&mut self, x: u16, y: u16, c: u32) {
		let base_index = ((y as usize) * (SCREEN_WIDTH as usize) + (x as usize)) * (PIXEL_BYTES as usize);
		// Write BGR directly without separate extractions
		self.pixels[base_index] = c as u8;
		self.pixels[base_index + 1] = (c >> 8) as u8;
		self.pixels[base_index + 2] = (c >> 16) as u8;
	}

	fn vblank(&mut self) {
        if let Ok(mut buffer) = self.shared_buffer.lock() {
            // Only copy NES screen size
            let size = SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize * PIXEL_BYTES as usize;
            buffer[..size].copy_from_slice(&self.pixels[..size]);
        }
        self.frame_ready.store(true, Ordering::Release);
	}

	fn copy_to_rgba_pixels(&self, _pixels: &mut [u8]) {
	}
}

pub struct Sdl2Renderer {
	texture: Texture<'static>,
	renderer: Canvas<Window>,
    is_portrait: bool,
    width: u32,
    height: u32,
    pixel_format: PixelFormatEnum,
}

impl Sdl2Renderer {
	pub fn new(sdl: Sdl, width: u32, height: u32) -> Self {
		let video_subsystem = sdl.video().unwrap();

		let window = video_subsystem.window(
			"nes-rust",
			width,
			height
		)
            .position_centered()
            .allow_highdpi()
            .resizable()
            .borderless()
            .build()
            .unwrap();

		let mut renderer = window
			.into_canvas()
			.accelerated()
			.build()
			.unwrap();
        
        // Logical size handling for Aspect Ratio
        let (w, h) = renderer.output_size().unwrap();
        let is_portrait = h > w;
        if is_portrait {
             // Heuristic for portrait mode scaling, mainly for NES on mobile
             // For DS (256x384), this might need adjustment, but 320 is close to 384.
             // Let's just use height if it's larger than typical NES.
             let logical_h = if height > 320 { height } else { 320 };
             let _ = renderer.set_logical_size(width, logical_h);
        } else {
             let _ = renderer.set_logical_size(width, height);
        }

        let pixel_format = if height == 384 {
            PixelFormatEnum::RGBA32 // DS
        } else {
            PixelFormatEnum::RGB24 // NES
        };

		let texture_creator = renderer.texture_creator();
		let texture_creator_pointer = &texture_creator as *const TextureCreator<WindowContext>;
		let texture = unsafe { &*texture_creator_pointer }
			.create_texture(
				pixel_format,
				TextureAccess::Streaming,
				width,
				height
			)
			.unwrap();

		Sdl2Renderer {
			texture: texture,
			renderer: renderer,
            is_portrait: is_portrait,
            width,
            height,
            pixel_format,
		}
	}

    pub fn draw(&mut self, pixels: &[u8]) {
        let pitch = self.width as usize * self.texture.query().format.byte_size_of_pixels(1);
        self.texture
			.update(None, pixels, pitch)
			.unwrap();

        let (w, h) = self.renderer.output_size().unwrap();
        let current_portrait = h > w;
        
        if current_portrait != self.is_portrait {
            self.is_portrait = current_portrait;
            if self.is_portrait {
                 let logical_h = if self.height > 320 { self.height } else { 320 };
                let _ = self.renderer.set_logical_size(self.width, logical_h);
            } else {
                let _ = self.renderer.set_logical_size(self.width, self.height);
            }
        }

		self.renderer.clear();
        
        if self.is_portrait {
            let dest_rect = sdl2::rect::Rect::new(0, 0, self.width, self.height);
            let _ = self.renderer.copy(&self.texture, None, Some(dest_rect));
        } else {
		    let _ = self.renderer.copy(&self.texture, None, None);
        }
        
		self.renderer.present();
    }
}
