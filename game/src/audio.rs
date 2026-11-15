//! Speaker output through the system's default audio device.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct Audio {
    _stream: Option<cpal::Stream>,
    pub queue: Arc<Mutex<VecDeque<f32>>>,
    pub rate: u32,
}

impl Audio {
    pub fn silent() -> Audio {
        Audio { _stream: None, queue: Arc::new(Mutex::new(VecDeque::new())), rate: 44100 }
    }

    pub fn new() -> Audio {
        let queue = Arc::new(Mutex::new(VecDeque::with_capacity(48000)));
        match open(queue.clone()) {
            Some((s, rate)) => Audio { _stream: Some(s), queue, rate },
            None => Audio { _stream: None, queue, rate: 44100 },
        }
    }

    /// Queue interleaved stereo samples, keeping the delay short.
    pub fn push(&self, samples: &mut Vec<f32>, volume: f32) {
        if self._stream.is_none() {
            samples.clear();
            return;
        }
        let mut q = self.queue.lock().unwrap();
        q.extend(samples.drain(..).map(|s| s * volume));
        let max = (self.rate as usize / 8) * 2; // ~125 ms
        while q.len() > max {
            q.pop_front();
        }
    }

    pub fn queued_seconds(&self) -> f32 {
        self.queue.lock().map(|q| q.len() as f32 / 2.0 / self.rate as f32).unwrap_or(0.0)
    }
}

fn open(q: Arc<Mutex<VecDeque<f32>>>) -> Option<(cpal::Stream, u32)> {
    let host = cpal::default_host();
    let dev = host.default_output_device()?;
    let cfg = dev.default_output_config().ok()?;
    let rate = cfg.sample_rate().0;
    let ch = cfg.channels() as usize;
    let sc: cpal::StreamConfig = cfg.config();
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(&dev, &sc, ch, q),
        cpal::SampleFormat::I16 => build::<i16>(&dev, &sc, ch, q),
        cpal::SampleFormat::U16 => build::<u16>(&dev, &sc, ch, q),
        _ => None,
    }?;
    stream.play().ok()?;
    Some((stream, rate))
}

fn build<T>(dev: &cpal::Device, sc: &cpal::StreamConfig, ch: usize, q: Arc<Mutex<VecDeque<f32>>>) -> Option<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let mut last = (0.0f32, 0.0f32);
    dev.build_output_stream(
        sc,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut q = q.lock().unwrap();
            for frame in data.chunks_mut(ch) {
                if q.len() >= 2 {
                    last = (q.pop_front().unwrap(), q.pop_front().unwrap());
                } else {
                    last = (last.0 * 0.995, last.1 * 0.995);
                }
                for (i, s) in frame.iter_mut().enumerate() {
                    let v = match i {
                        0 => last.0,
                        1 => last.1,
                        _ => (last.0 + last.1) * 0.5,
                    };
                    *s = T::from_sample(v);
                }
            }
        },
        |e: cpal::StreamError| eprintln!("audio error: {e}"),
        None,
    )
    .ok()
}
