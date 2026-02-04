pub const SCREEN_WIDTH: u32 = 256;
pub const SCREEN_HEIGHT: u32 = 240;
pub const PIXEL_BYTES: u32 = 3;
// Capacity for largest supported screen (DS: 256x384x4)
pub const PIXELS_CAPACITY: usize = 256 * 384 * 4;

pub trait Display {
	fn render_pixel(&mut self, x: u16, y: u16, c: u32);
	fn vblank(&mut self);
	fn copy_to_rgba_pixels(&self, pixels: &mut [u8]);
}
