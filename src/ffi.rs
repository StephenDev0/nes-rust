use std::slice;
use std::os::raw::{c_int, c_uchar, c_float, c_char};
use std::ffi::CStr;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::VecDeque;
use std::path::Path;

use sdl2::Sdl;

use crate::Nes;
use crate::rom::Rom;
use crate::default_input::DefaultInput;
use crate::default_display::DefaultDisplay;
use crate::default_audio::DefaultAudio;
use crate::button::{Button, Event};
use crate::input::Input;
use crate::sdl_backend::input::Sdl2Input;
use crate::sdl_backend::display::{SharedDisplay, Sdl2Renderer};
use crate::sdl_backend::audio::Sdl2Audio;
use crate::display::{PIXELS_CAPACITY, SCREEN_WIDTH, SCREEN_HEIGHT, PIXEL_BYTES};
use crate::ds::emulator::DsEmulator;
use crate::ds::emu::input::Keys;

lazy_static! {
    static ref INPUT_EVENTS: Mutex<VecDeque<(Button, Event)>> = Mutex::new(VecDeque::new());
    static ref TOUCH_EVENT: Mutex<Option<(u16, u16, bool)>> = Mutex::new(None);
}

struct IosInput;
impl IosInput {
    fn new() -> Self { IosInput }
}
impl Input for IosInput {
    fn get_input(&mut self) -> Option<(Button, Event)> {
        if let Ok(mut queue) = INPUT_EVENTS.lock() {
            return queue.pop_front();
        }
        None
    }
    fn press(&mut self, _button: Button) {}
    fn release(&mut self, _button: Button) {}
}

fn map_button(id: c_int) -> Option<Button> {
    match id {
        0 => Some(Button::Joypad1A),
        1 => Some(Button::Joypad1B),
        2 => Some(Button::Select),
        3 => Some(Button::Start),
        4 => Some(Button::Joypad1Up),
        5 => Some(Button::Joypad1Down),
        6 => Some(Button::Joypad1Left),
        7 => Some(Button::Joypad1Right),
        8 => Some(Button::Reset),
        9 => Some(Button::Poweroff),
        10 => Some(Button::X),
        11 => Some(Button::Y),
        12 => Some(Button::L),
        13 => Some(Button::R),
        _ => None,
    }
}

// Virtual button input (global, thread-safe)
#[no_mangle]
pub extern "C" fn set_virtual_button_state(button_id: c_int, pressed: c_int) {
    if let Some(button) = map_button(button_id) {
        let event = if pressed != 0 { Event::Press } else { Event::Release };
        if let Ok(mut queue) = INPUT_EVENTS.lock() {
            queue.push_back((button, event));
        }
    }
}

// Touch input for DS (global, thread-safe)
// Coordinates should be in emulator screen space (256x384)
#[no_mangle]
pub extern "C" fn nes_touch(_state_ptr: *mut EmuState, x: c_int, y: c_int, pressed: c_int) {
    if let Ok(mut touch) = TOUCH_EVENT.lock() {
        if pressed != 0 {
            if y >= 192 && y < 384 && x >= 0 && x < 256 {
                let ds_x = x as u16;
                let ds_y = (y - 192) as u16;
                *touch = Some((ds_x, ds_y, true));
            }
        } else {
            *touch = Some((0, 0, false));
        }
    }
}

pub enum EmuCore {
    Nes(Nes),
    Ds(DsEmulator),
}

pub struct EmuState {
    pub core: Arc<Mutex<Option<EmuCore>>>, 
    pub renderer: Option<Sdl2Renderer>,
    pub shared_buffer: Arc<Mutex<[u8; PIXELS_CAPACITY]>>,
    pub frame_ready: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub sdl_context: Option<Sdl>,
    pub width: u32,
    pub height: u32,
    pub save_path: String,
}

#[no_mangle]
pub extern "C" fn initEmu(rom_path: *const c_char) -> *mut EmuState {
    let c_str = unsafe { CStr::from_ptr(rom_path) };
    let filename = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let save_path = format!("{}.save", filename);

    let sdl = match sdl2::init() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    // Configure SDL hints for iOS
    sdl2::hint::set("SDL_IOS_ORIENTATIONS", "Portrait LandscapeLeft LandscapeRight PortraitUpsideDown");
    sdl2::hint::set("SDL_IOS_HIDE_HOME_INDICATOR", "0");
    
    let audio_subsystem = match sdl.audio() {
        Ok(a) => a,
        Err(_) => return std::ptr::null_mut(),
    };

    let shared_buffer = Arc::new(Mutex::new([0u8; PIXELS_CAPACITY]));
    let frame_ready = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));

    let is_ds = filename.to_lowercase().ends_with(".nds");

    let core;
    let width;
    let height;

    if is_ds {
        width = 256;
        height = 384;
        let emu = match DsEmulator::new(Path::new(filename), None, None, None, &audio_subsystem, Path::new(&save_path)) {
             Ok(e) => e,
             Err(_) => return std::ptr::null_mut(),
        };
        core = Some(EmuCore::Ds(emu));
    } else {
        width = SCREEN_WIDTH;
        height = SCREEN_HEIGHT;

        let mut file = match File::open(filename) {
            Ok(f) => f,
            Err(_) => return std::ptr::null_mut(),
        };
        
        let mut contents = vec![];
        if file.read_to_end(&mut contents).is_err() {
            return std::ptr::null_mut();
        }
        
        let rom = match Rom::new(contents) {
            Some(r) => r,
            None => return std::ptr::null_mut(),
        };

        let input = Box::new(IosInput::new());
        let display = Box::new(SharedDisplay::new(shared_buffer.clone(), frame_ready.clone()));
        let audio = Box::new(Sdl2Audio::new(audio_subsystem));
        let mut nes = Nes::new(input, display, audio);
        nes.set_rom(rom);
        nes.bootup();
        core = Some(EmuCore::Nes(nes));
    }

    let renderer = Sdl2Renderer::new(sdl.clone(), width, height);
    
    let state = Box::into_raw(Box::new(EmuState {
        core: Arc::new(Mutex::new(core)),
        renderer: Some(renderer),
        shared_buffer,
        frame_ready,
        paused,
        sdl_context: Some(sdl),
        width,
        height,
        save_path,
    }));
    state
}

#[no_mangle]
pub extern "C" fn runEmuLoop(state_ptr: *mut EmuState) {
    if state_ptr.is_null() { return; }
    let state = unsafe { &mut *state_ptr };
    
    let mut last_save_check = std::time::Instant::now();

    loop {
        if state.paused.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        let mut core_lock = state.core.lock().unwrap();
        if let Some(core) = &mut *core_lock {
            match core {
                EmuCore::Nes(nes) => {
                    nes.step_frame();
                    if !nes.is_power_on() {
                        break;
                    }
                },
                EmuCore::Ds(ds) => {
                    // ... input processing ...
                    if let Ok(mut touch_lock) = TOUCH_EVENT.lock() {
                        if let Some((x, y, pressed)) = touch_lock.take() {
                            if pressed {
                                ds.set_touch_pos(x, y);
                            } else {
                                ds.end_touch();
                            }
                        }
                    }

                    if let Ok(mut queue) = INPUT_EVENTS.lock() {
                        while let Some((button, event)) = queue.pop_front() {
                            let mask = match button {
                                Button::Joypad1A => 1 << 0,
                                Button::Joypad1B => 1 << 1,
                                Button::Select => 1 << 2,
                                Button::Start => 1 << 3,
                                Button::Joypad1Right => 1 << 4,
                                Button::Joypad1Left => 1 << 5,
                                Button::Joypad1Up => 1 << 6,
                                Button::Joypad1Down => 1 << 7,
                                Button::R => 1 << 8,
                                Button::L => 1 << 9,
                                Button::X => 1 << 16,
                                Button::Y => 1 << 17,
                                _ => 0,
                            };
                            if mask != 0 {
                                let keys = Keys::from_bits_truncate(mask);
                                match event {
                                    Event::Press => ds.emu.press_keys(keys),
                                    Event::Release => ds.emu.release_keys(keys),
                                }
                            }
                        }
                    }

                    ds.step();
                    
                    // Periodic save persistence
                    if last_save_check.elapsed() > Duration::from_secs(2) {
                        if ds.save_dirty() {
                            let data = ds.save_data();
                            if let Ok(mut file) = File::create(&state.save_path) {
                                let _ = file.write_all(data);
                                ds.mark_save_flushed();
                            }
                        }
                        last_save_check = std::time::Instant::now();
                    }

                    let fb = ds.emu.gpu.renderer_2d().framebuffer();
                    if let Ok(mut buffer) = state.shared_buffer.lock() {
                        let mut offset = 0;
                        for i in 0..2 {
                             let screen = &fb[i];
                             for &px in screen.iter() {
                                    if offset + 4 > PIXELS_CAPACITY { break; }
                                    let r = (px & 0xFF) as u8;
                                    let g = ((px >> 8) & 0xFF) as u8;
                                    let b = ((px >> 16) & 0xFF) as u8;
                                    let a = ((px >> 24) & 0xFF) as u8;
                                    
                                    buffer[offset] = r;
                                    buffer[offset+1] = g;
                                    buffer[offset+2] = b;
                                    buffer[offset+3] = a;
                                    offset += 4;
                             }
                        }
                    }
                    state.frame_ready.store(true, Ordering::Release);
                }
            }
        } else {
            break;
        }
        drop(core_lock);
        // Speed is now handled by DsAudioBackend blocking. 
        // No sleep needed here for DS if audio is running.
    }
}
#[no_mangle]
pub extern "C" fn setEmuPaused(state_ptr: *mut EmuState, paused: c_int) {
    if state_ptr.is_null() { return; }
    let state = unsafe { &mut *state_ptr };
    state.paused.store(paused != 0, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn nes_is_ds(state_ptr: *mut EmuState) -> c_int {
    if state_ptr.is_null() { return 0; }
    let state = unsafe { &mut *state_ptr };
    let core_lock = state.core.lock().unwrap();
    if let Some(EmuCore::Ds(_)) = &*core_lock {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn renderFrame(state_ptr: *mut EmuState) {
    if state_ptr.is_null() { return; }
    let state = unsafe { &mut *state_ptr };

    if state.frame_ready.swap(false, Ordering::Acquire) {
        if let Ok(buffer) = state.shared_buffer.lock() {
            if let Some(renderer) = &mut state.renderer {
                let size = state.width as usize * state.height as usize * 4;
                renderer.draw(&buffer[..size]);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn stopEmu(state_ptr: *mut EmuState) {
    if state_ptr.is_null() { return; }
    let state = unsafe { &mut *state_ptr };
    let mut core_lock = state.core.lock().unwrap();
    if let Some(core) = &mut *core_lock {
        match core {
            EmuCore::Nes(nes) => nes.press_button(Button::Poweroff),
            EmuCore::Ds(_) => { 
                *core_lock = None;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn resetEmu(state_ptr: *mut EmuState) {
    if state_ptr.is_null() { return; }
    let state = unsafe { &mut *state_ptr };
    let mut core_lock = state.core.lock().unwrap();
     if let Some(core) = &mut *core_lock {
        match core {
            EmuCore::Nes(nes) => nes.reset(),
             EmuCore::Ds(_) => { }
        }
    }
}

#[no_mangle]
pub extern "C" fn cleanupEmu(state_ptr: *mut EmuState) {
    if state_ptr.is_null() { return; }
    unsafe {
        let _ = Box::from_raw(state_ptr);
    }
}

#[no_mangle]
pub extern "C" fn startEmu(rom_path: *const c_char, _state_path: *const c_char, _slot_count: c_int, _initial_slot: c_int) -> c_int {
    1
}

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
    if let Some(rom) = Rom::new(data_slice.to_vec()) {
        nes.set_rom(rom);
        nes.bootup();
    }
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
    if let Some(button) = map_button(button_id) {
        if pressed != 0 {
            nes.press_button(button);
        } else {
            nes.release_button(button);
        }
    }
}