use display::{
	Display,
	PIXEL_BYTES,
	PIXELS_CAPACITY,
	SCREEN_HEIGHT,
	SCREEN_WIDTH
};

pub struct DefaultDisplay {
	pixels: Vec<u8>
}

impl DefaultDisplay {
	pub fn new() -> Self {
		DefaultDisplay {
			pixels: vec![0; PIXELS_CAPACITY]
		}
	}
}

impl Display for DefaultDisplay {
	#[inline(always)]
	fn render_pixel(&mut self, x: u16, y: u16, c: u32) {
		let base_index = ((y as usize) * (SCREEN_WIDTH as usize) + (x as usize)) * (PIXEL_BYTES as usize);
		// Write BGR directly without separate extractions
		self.pixels[base_index] = c as u8;
		self.pixels[base_index + 1] = (c >> 8) as u8;
		self.pixels[base_index + 2] = (c >> 16) as u8;
	}

	fn vblank(&mut self) {
	}

	fn copy_to_rgba_pixels(&self, pixels: &mut [u8]) {
		for y in 0..SCREEN_HEIGHT {
			for x in 0..SCREEN_WIDTH {
				let base_index = (y * SCREEN_WIDTH + x) as usize;
				pixels[base_index * 4 + 0] = self.pixels[base_index * 3 + 0];
				pixels[base_index * 4 + 1] = self.pixels[base_index * 3 + 1];
				pixels[base_index * 4 + 2] = self.pixels[base_index * 3 + 2];
				pixels[base_index * 4 + 3] = 255;
			}
		}
	}
}
