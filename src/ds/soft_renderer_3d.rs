// Remove #![feature(portable_simd)] since it's a library crate attribute and needs to be in root, or allowed if using nightly.
// But we are in a sub-module. We assume the root crate enables it or we use stable compatible code.
// dust-core uses nightly features. We enabled 'nightly' toolchain.
// The error `use of unstable library feature` suggests we need to enable it in the crate root `src/lib.rs`.

use crate::ds::dust_core::{
    gpu::{
        engine_3d::{
            Polygon, RendererTx, RenderingState as CoreRenderingState, ScreenVertex, SoftRendererRx,
            RenderingControl,
        },
        Scanline, SCREEN_HEIGHT,
    },
    utils::Bytes,
};
use crate::ds::dust_soft_3d::{Renderer, RenderingData};
use std::{
    cell::UnsafeCell,
    hint,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    thread,
};

struct SharedData {
    rendering_data: Box<UnsafeCell<RenderingData>>,
    scanline_buffer: Box<UnsafeCell<[Scanline<u32>; SCREEN_HEIGHT]>>,
    processing_scanline: AtomicU8,
    stopped: AtomicBool,
}

unsafe impl Sync for SharedData {}

pub struct Tx {
    shared_data: Arc<SharedData>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Tx {
    fn wait_for_frame_end(&self) {
        while {
            let processing_scanline = self.shared_data.processing_scanline.load(Ordering::Acquire);
            processing_scanline == u8::MAX || processing_scanline < SCREEN_HEIGHT as u8
        } {
            hint::spin_loop();
        }
    }
}

impl RendererTx for Tx {
    fn set_capture_enabled(&mut self, _capture_enabled: bool) {}

    fn swap_buffers(
        &mut self,
        vert_ram: &[ScreenVertex],
        poly_ram: &[Polygon],
        state: &CoreRenderingState,
    ) {
        self.wait_for_frame_end();
        unsafe { &mut *self.shared_data.rendering_data.get() }.prepare(vert_ram, poly_ram, state);
    }

    fn repeat_last_frame(&mut self, state: &CoreRenderingState) {
        self.wait_for_frame_end();
        unsafe { &mut *self.shared_data.rendering_data.get() }.repeat_last_frame(state);
    }

    fn start_rendering(
        &mut self,
        texture: &Bytes<0x8_0000>,
        tex_pal: &Bytes<0x1_8000>,
        state: &CoreRenderingState,
    ) {
        unsafe { &mut *self.shared_data.rendering_data.get() }.copy_vram(texture, tex_pal, state);

        self.shared_data
            .processing_scanline
            .store(u8::MAX, Ordering::Release);
        self.thread.as_ref().unwrap().thread().unpark();
    }

    fn skip_rendering(&mut self) {}
}

impl Drop for Tx {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.shared_data.stopped.store(true, Ordering::Relaxed);
            thread.thread().unpark();
            let _ = thread.join();
            self.shared_data
                .processing_scanline
                .store(SCREEN_HEIGHT as u8, Ordering::Relaxed);
        }
    }
}

#[derive(Clone)]
pub struct Rx {
    next_scanline: u8,
    shared_data: Arc<SharedData>,
}

impl Rx {
    fn wait_for_line(&self, line: u8) {
        while {
            let processing_scanline = self.shared_data.processing_scanline.load(Ordering::Acquire);
            processing_scanline == u8::MAX || processing_scanline <= line
        } {
            hint::spin_loop();
        }
    }
}

impl SoftRendererRx for Rx {
    fn start_frame(&mut self) {
        self.next_scanline = 0;
    }

    fn read_scanline(&mut self) -> &Scanline<u32> {
        self.wait_for_line(self.next_scanline);
        let result =
            unsafe { &(&*self.shared_data.scanline_buffer.get())[self.next_scanline as usize] };
        self.next_scanline += 1;
        result
    }

    fn skip_scanline(&mut self) {
        self.next_scanline += 1;
    }
}

fn create_rendering_data() -> RenderingData {
    RenderingData {
        control: RenderingControl(0),
        w_buffering: false,
        alpha_test_ref: 0,
        clear_poly_id: 0,
        clear_image_offset: [0; 2],
        clear_depth: 0,
        fog_offset: 0,
        fog_densities: [0; 0x22],
        rear_plane_fog_enabled: false,
        clear_color: unsafe { std::mem::zeroed() }, 
        fog_color: unsafe { std::mem::zeroed() },
        edge_colors: unsafe { std::mem::zeroed() },
        toon_colors: unsafe { std::mem::zeroed() },
        texture: Bytes::new([0; 0x80000]),
        tex_pal: Bytes::new([0; 0x20000]),
        vert_ram: unsafe { std::mem::zeroed() },
        poly_ram: unsafe { std::mem::zeroed() },
        poly_ram_level: 0,
    }
}

pub fn init() -> (Tx, Rx) {
    let shared_data = Arc::new(unsafe {
        SharedData {
            rendering_data: Box::new(UnsafeCell::new(create_rendering_data())),
            // scanline_buffer contains Scanline<u32, 256>. This is 1KB per line. 192 lines. 
            // Box::new_zeroed() is likely fine here as it's just array.
            // Using box new with value from emu_utils::containers::new_zeroed_box would be best but it's hidden.
            // Using Box::new_zeroed::<T>() from std (nightly).
            scanline_buffer: std::mem::transmute(Box::<[Scanline<u32>; SCREEN_HEIGHT]>::new_zeroed()),
            processing_scanline: AtomicU8::new(SCREEN_HEIGHT as u8),
            stopped: AtomicBool::new(false),
        }
    });
    let rx = Rx {
        next_scanline: 0,
        shared_data: Arc::clone(&shared_data),
    };
    (
        Tx {
            shared_data: Arc::clone(&shared_data),
            thread: Some(
                thread::Builder::new()
                    .name("3D rendering".to_owned())
                    .spawn(move || {
                        let mut raw_renderer = Renderer::new();
                        loop {
                            if shared_data.stopped.load(Ordering::Relaxed) {
                                return;
                            }
                            if shared_data
                                .processing_scanline
                                .compare_exchange(u8::MAX, 0, Ordering::Acquire, Ordering::Acquire)
                                .is_ok()
                            {
                                let rendering_data = unsafe { &*shared_data.rendering_data.get() };
                                raw_renderer.start_frame(rendering_data);
                                raw_renderer.render_line(0, rendering_data);
                                for y in 0..192 {
                                    let scanline =
                                        &mut unsafe { &mut *shared_data.scanline_buffer.get() }
                                            [y as usize];
                                    if y < 191 {
                                        raw_renderer.render_line(y + 1, rendering_data);
                                    }
                                    raw_renderer.postprocess_line(y, scanline, rendering_data);
                                    let _ = shared_data.processing_scanline.compare_exchange(
                                        y,
                                        y + 1,
                                        Ordering::Release,
                                        Ordering::Relaxed,
                                    );
                                }
                            } else {
                                thread::park();
                            }
                        }
                    })
                    .expect("couldn't spawn 3D rendering thread"),
            ),
        },
        rx,
    )
}
