use crate::ds::dust_core::audio::{Backend, OutputSample};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::AudioSubsystem;

pub struct DsAudioBackend {
    pub device: AudioQueue<i16>,
}

impl DsAudioBackend {
    pub fn new(subsystem: &AudioSubsystem) -> Self {
        let spec = AudioSpecDesired {
            freq: Some(44100), 
            channels: Some(2), // Stereo
            samples: Some(1024),
        };

        let device = subsystem.open_queue(None, &spec).unwrap();
        device.resume();

        DsAudioBackend { device }
    }
}

impl Backend for DsAudioBackend {
    fn handle_sample_chunk(&mut self, samples: &mut Vec<[OutputSample; 2]>) {
        if samples.is_empty() {
            return;
        }

        // Throttle emulation speed by blocking if the SDL audio queue is too full.
        // Target: ~4 frames of audio (approx 16KB).
        while self.device.size() > 16384 {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let mut buffer = Vec::with_capacity(samples.len() * 2);
        for channels in samples.iter() {
            for &sample in channels.iter() {
                // sample is 0..1023. Center is 512.
                // Scale up to 16-bit range.
                let val = (sample as i32 - 512) * 60;
                buffer.push(val.clamp(-32768, 32767) as i16);
            }
        }

        let _ = self.device.queue_audio(&buffer);
        samples.clear();
    }
}
