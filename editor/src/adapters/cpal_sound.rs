//! Speaker output through the system's default audio device (opened on first use).
use crate::ports::SoundOut;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct Speakers {
    stream: Option<cpal::Stream>,
    queue: Arc<Mutex<VecDeque<f32>>>,
    rate: u32,
}

impl Speakers {
    pub fn new() -> Speakers {
        let queue = Arc::new(Mutex::new(VecDeque::with_capacity(48000)));
        match open(queue.clone()) {
            Some((s, rate)) => Speakers { stream: Some(s), queue, rate },
            None => Speakers { stream: None, queue, rate: 44100 },
        }
    }
}

impl SoundOut for Speakers {
    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn push(&self, samples: &[f32], volume: f32) {
        if self.stream.is_none() {
            return;
        }
        if let Ok(mut q) = self.queue.lock() {
            q.extend(samples.iter().map(|s| s * volume));
            let max = (self.rate as usize / 8) * 2;
            while q.len() > max {
                q.pop_front();
            }
        }
    }

    fn queued_seconds(&self) -> f32 {
        if self.stream.is_none() {
            return 1.0;
        }
        self.queue.lock().map(|q| q.len() as f32 / 2.0 / self.rate as f32).unwrap_or(1.0)
    }
}

fn open(q: Arc<Mutex<VecDeque<f32>>>) -> Option<(cpal::Stream, u32)> {
    let dev = cpal::default_host().default_output_device()?;
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
    dev.build_output_stream(
        sc,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let Ok(mut q) = q.lock() else { return };
            for frame in data.chunks_mut(ch) {
                let (l, r) = if q.len() >= 2 { (q.pop_front().unwrap_or(0.0), q.pop_front().unwrap_or(0.0)) } else { (0.0, 0.0) };
                for (i, s) in frame.iter_mut().enumerate() {
                    *s = T::from_sample(match i {
                        0 => l,
                        1 => r,
                        _ => (l + r) * 0.5,
                    });
                }
            }
        },
        |e: cpal::StreamError| eprintln!("audio error: {e}"),
        None,
    )
    .ok()
}
