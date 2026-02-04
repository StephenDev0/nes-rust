use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::AudioSubsystem;
use std::time::Duration;
use std::thread;

use crate::audio::Audio;

pub struct Sdl2Audio {
	device: AudioQueue<f32>,
    buffer: Vec<f32>,
}

impl Sdl2Audio {
	pub fn new(subsystem: AudioSubsystem) -> Self {
		let spec = AudioSpecDesired {
			freq: Some(44100),
			channels: Some(1),
			samples: Some(1024)
		};
        
        let device = subsystem.open_queue(
            None,
            &spec
        ).unwrap();
        
        device.resume();
        
		Sdl2Audio {
			device,
            buffer: Vec::with_capacity(1024),
		}
	}
}

impl Audio for Sdl2Audio {
	fn push(&mut self, value: f32) {
        self.buffer.push(value * 0.25); // Volume
        if self.buffer.len() >= 512 {
            // Check queue size to sync emulation speed
            // Target buffer: ~2-3 frames (~6KB - 9KB).
            // If queue is larger, yield or sleep to let it drain.
            // 8192 * 2 = 16384 bytes (~92ms at 44.1kHz)
            while self.device.size() > 8192 * 2 {
                if self.device.size() > 8192 * 4 {
                    thread::sleep(Duration::from_millis(1));
                } else {
                    thread::yield_now();
                }
            }
            
            let _ = self.device.queue_audio(&self.buffer);
            self.buffer.clear();
        }
	}

	fn copy_sample_buffer(&mut self, _sample_buffer: &mut [f32]) {
	}
}
