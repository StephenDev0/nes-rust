use std::slice;
use std::os::raw::{c_int, c_uchar, c_float};
use crate::Nes;
use crate::rom::Rom;
use crate::default_input::DefaultInput;
use crate::default_display::DefaultDisplay;
use crate::default_audio::DefaultAudio;
use crate::button::Button;

#[no_mangle]
pub extern "C" fn nes_create() -> *mut Nes {
    let input = Box::new(DefaultInput::new());
    let display = Box::new(DefaultDisplay::new());
    let audio = Box::new(DefaultAudio::new());
    let nes = Nes::new(input, display, audio);
    Box::into_raw(Box::new(nes))
}

#[no_mangle]
pub extern "C" fn nes_destroy(nes_ptr: *mut Nes) {
    if nes_ptr.is_null() { return; }
    unsafe {
        let _ = Box::from_raw(nes_ptr);
    }
}

#[no_mangle]
pub extern "C" fn nes_load_rom(nes_ptr: *mut Nes, data: *const c_uchar, len: usize) {
    let nes = unsafe { &mut *nes_ptr };
    let data_slice = unsafe { slice::from_raw_parts(data, len) };
    let rom = Rom::new(data_slice.to_vec());
    nes.set_rom(rom);
    nes.bootup();
}

#[no_mangle]
pub extern "C" fn nes_reset(nes_ptr: *mut Nes) {
    let nes = unsafe { &mut *nes_ptr };
    nes.reset();
}

#[no_mangle]
pub extern "C" fn nes_step_frame(nes_ptr: *mut Nes) {
    let nes = unsafe { &mut *nes_ptr };
    nes.step_frame();
}

#[no_mangle]
pub extern "C" fn nes_get_pixels(nes_ptr: *mut Nes, buffer: *mut c_uchar, len: usize) {
    let nes = unsafe { &mut *nes_ptr };
    let buffer_slice = unsafe { slice::from_raw_parts_mut(buffer, len) };
    nes.copy_pixels(buffer_slice);
}

#[no_mangle]
pub extern "C" fn nes_get_audio_samples(nes_ptr: *mut Nes, buffer: *mut c_float, len: usize) {
    let nes = unsafe { &mut *nes_ptr };
    let buffer_slice = unsafe { slice::from_raw_parts_mut(buffer, len) };
    nes.copy_sample_buffer(buffer_slice);
}

#[no_mangle]
pub extern "C" fn nes_input(nes_ptr: *mut Nes, button_id: c_int, pressed: c_int) {
    let nes = unsafe { &mut *nes_ptr };
    let button = match button_id {
        0 => Button::Joypad1A,
        1 => Button::Joypad1B,
        2 => Button::Select,
        3 => Button::Start,
        4 => Button::Joypad1Up,
        5 => Button::Joypad1Down,
        6 => Button::Joypad1Left,
        7 => Button::Joypad1Right,
        8 => Button::Reset,
        9 => Button::Poweroff,
        _ => return,
    };
    
    if pressed != 0 {
        nes.press_button(button);
    } else {
        nes.release_button(button);
    }
}
