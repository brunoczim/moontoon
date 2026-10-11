use std::{
    fmt,
    mem,
    num::NonZero,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::Error;

pub trait AudioStream: Send + Sync + 'static {
    fn at(&self, seconds: f64, channel: u16) -> Result<f64, Error>;

    fn channels(&self) -> NonZero<u16>;

    fn sample_rate(&self) -> NonZero<u32>;

    fn duration(&self) -> Duration;
}

pub struct Player {
    playing: Option<Arc<Shared>>,
    sink: rodio::MixerDeviceSink,
    inner: rodio::Player,
}

impl Player {
    pub fn connect() -> Result<Self, Error> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink()?;
        sink.log_on_drop(false);
        let inner = rodio::Player::connect_new(&sink.mixer());
        Ok(Self { playing: None, sink, inner })
    }

    pub fn play(&mut self, stream: impl AudioStream) {
        let shared = Arc::new(Shared { result: Mutex::new(Ok(())) });
        let bridge = RodioBridge::new(stream, shared.clone());
        self.playing = Some(shared);
        self.inner.append(bridge);
        self.inner.play();
    }

    pub fn take_stream_result(&self) -> Result<(), Error> {
        let Some(shared) = self.playing.as_ref() else { return Ok(()) };
        let mut guard = shared.result.lock().expect("poisoned lock");
        mem::replace(&mut *guard, Ok(()))
    }

    pub fn wait_until_end(&self) -> Result<(), Error> {
        self.inner.sleep_until_end();
        self.take_stream_result()
    }
}

impl fmt::Debug for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Player")
            .field("playing", &self.playing)
            .field("sink", &self.sink)
            .field("inner", &(&self.inner as *const _))
            .finish()
    }
}

#[derive(Debug)]
struct Shared {
    result: Mutex<Result<(), Error>>,
}

#[derive(Debug)]
struct RodioBridge<S> {
    stream: S,
    time: f64,
    channel: u16,
    shared: Arc<Shared>,
    error: bool,
}

impl<S> RodioBridge<S>
where
    S: AudioStream,
{
    pub fn new(stream: S, shared: Arc<Shared>) -> Self {
        Self { stream, time: 0.0, channel: 0, shared, error: false }
    }
}

impl<S> Iterator for RodioBridge<S>
where
    S: AudioStream,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.time >= self.stream.duration().as_secs_f64() {
            return None;
        }
        if self.error {
            return None;
        }

        let sample = match self.stream.at(self.time, self.channel) {
            Ok(x) => x,
            Err(e) => {
                let mut guard =
                    self.shared.result.lock().expect("poisoned lock");
                *guard = Err(e);
                self.error = true;
                return None;
            },
        };
        self.time += 1.0 / f64::from(self.stream.sample_rate().get());
        self.channel = (self.channel + 1) % self.stream.channels().get();
        Some(sample as rodio::Sample)
    }
}

impl<S> rodio::Source for RodioBridge<S>
where
    S: AudioStream,
{
    fn current_span_len(&self) -> Option<usize> {
        if self.time >= self.stream.duration().as_secs_f64() || self.error {
            Some(0)
        } else {
            None
        }
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.stream.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.stream.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.stream.duration())
    }
}
