use derivative::Derivative;
use faad2::{version, Decoder};
use log;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use shared::edi::msc::{AACPResult, AudioFormat};
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

fn calc_rms(samples: &[f32], channels: usize) -> (f32, f32) {
    let mut sum_l = 0.0;
    let mut sum_r = 0.0;
    let mut count_l = 0;
    let mut count_r = 0;

    for (i, sample) in samples.iter().enumerate() {
        if i % channels == 0 {
            sum_l += sample * sample;
            count_l += 1;
        } else {
            sum_r += sample * sample;
            count_r += 1;
        }
    }

    let rms_l = (sum_l / count_l as f32).sqrt();
    let rms_r = (sum_r / count_r as f32).sqrt();
    (rms_l, rms_r)
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum AudioEvent {
    LevelsUpdated(AudioLevels),
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct AudioLevels {
    pub peak: (f32, f32),
    pub peak_clamped: (f32, f32),
    pub rms: (f32, f32),

    pub peak_smooth: (f32, f32),
    pub rms_smooth: (f32, f32),

    #[derivative(Debug = "ignore")]
    last_update: Instant,
}

impl AudioLevels {
    pub fn new() -> Self {
        Self {
            peak: (0.0, 0.0),
            peak_clamped: (0.0, 0.0),
            rms: (0.0, 0.0),

            peak_smooth: (0.0, 0.0),
            rms_smooth: (0.0, 0.0),

            last_update: Instant::now(),
        }
    }

    pub fn smooth(
        current: (f32, f32),
        previous: (f32, f32),
        dt: f32,
        decay_per_second: f32,
    ) -> (f32, f32) {
        fn smooth_channel(new: f32, prev: f32, dt: f32, decay: f32) -> f32 {
            if new > prev {
                new
            } else {
                let decayed = prev - decay * dt;
                decayed.max(new)
            }
        }

        (
            smooth_channel(current.0, previous.0, dt, decay_per_second),
            smooth_channel(current.1, previous.1, dt, decay_per_second),
        )
    }

    pub fn feed(&mut self, channels: usize, samples: &[f32]) {
        let count = samples.len() / channels;
        let top_n = 16;

        let mut peaks_l = Vec::with_capacity(count);
        let mut peaks_r = Vec::with_capacity(count);

        let mut sum_l: f32 = 0.0;
        let mut sum_r: f32 = 0.0;

        for (i, sample) in samples.iter().enumerate() {
            let abs = sample.abs();
            if i % channels == 0 {
                peaks_l.push(abs);
                sum_l += sample * sample;
            } else {
                peaks_r.push(abs);
                sum_r += sample * sample;
            }
        }

        peaks_l.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        peaks_r.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        let peak_l = peaks_l.iter().take(top_n).copied().sum::<f32>() / peaks_l.len().min(top_n) as f32;
        let peak_r = peaks_r.iter().take(top_n).copied().sum::<f32>() / peaks_r.len().min(top_n) as f32;

        let rms_l = (sum_l / count as f32).sqrt();
        let rms_r = (sum_r / count as f32).sqrt();

        self.peak = (peak_l, peak_r);
        self.peak_clamped = (peak_l.clamp(0.0, 1.0), peak_r.clamp(0.0, 1.0));
        self.rms = (rms_l, rms_r);

        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        self.peak_smooth = Self::smooth(self.peak, self.peak_smooth, dt, 0.05);
        self.rms_smooth = Self::smooth(self.rms, self.rms_smooth, dt, 0.1);

        log::debug!("{:?}", self);
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AudioDecoder {
    scid: u8,
    asc: Vec<u8>,
    audio_format: AudioFormat,
    #[derivative(Debug = "ignore")]
    decoder: Decoder,
    #[derivative(Debug = "ignore")]
    _stream: OutputStream,
    #[derivative(Debug = "ignore")]
    sink: Arc<Mutex<Sink>>,
    #[derivative(Debug = "ignore")]
    tx: UnboundedSender<AudioEvent>,
    levels: AudioLevels,
}

impl AudioDecoder {
    #[allow(dead_code)]
    pub fn version() -> &'static str {
        version().0
    }

    pub fn new(
        scid: u8,
        initial_audio_format: AudioFormat,
        tx: UnboundedSender<AudioEvent>,
    ) -> Self {
        let asc = initial_audio_format.asc.clone();
        let decoder = Decoder::new(&asc).expect("Failed to create initial decoder");

        let (stream, handle) = OutputStream::try_default().expect("Error creating output stream");
        let sink = Arc::new(Mutex::new(
            Sink::try_new(&handle).expect("Error creating sink"),
        ));

        Self {
            scid,
            asc,
            audio_format: initial_audio_format,
            decoder,
            _stream: stream,
            sink,
            tx,
            levels: AudioLevels::new(),
        }
    }

    fn reconfigure(&mut self, new_audio_format: &AudioFormat) -> Result<(), Error> {
        log::info!(
            "Reconfiguring audio decoder for format: {:?}",
            new_audio_format
        );
        match Decoder::new(&new_audio_format.asc) {
            Ok(new_decoder) => {
                self.decoder = new_decoder;
                self.audio_format = new_audio_format.clone();
                self.asc = new_audio_format.asc.clone();
                self.sink.lock().unwrap().stop();
                Ok(())
            }
            Err(_e) => {
                // log::error!("Failed to reconfigure audio decoder: {}", e);
                Err(Error::new(ErrorKind::Other, "Decoder error"))
            }
        }
    }

    /*
    fn fade(vol: f32, duration_ms: u64) {
        // NOTE: implement generic fade logic here
    }
    */

    pub fn feed(&mut self, aac_result: &AACPResult) {
        if let Some(new_audio_format) = &aac_result.audio_format {
            if new_audio_format != &self.audio_format {
                if self.reconfigure(new_audio_format).is_err() {
                    log::warn!("Decoder reconfiguration failed, skipping audio data");
                    return;
                }
            }
        }

        if aac_result.scid != self.scid {
            log::info!("Changed SCID: {} > {}", self.scid, aac_result.scid);

            /**/
            self.sink.lock().unwrap().set_volume(0.0);

            let sink_clone = Arc::clone(&self.sink);
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(50));

                let fade_duration = Duration::from_millis(200);
                let steps = 20; // Update volume every 10ms
                let step_duration = fade_duration / steps;
                let volume_step = 1.0 / steps as f32;

                for i in 1..=steps {
                    thread::sleep(step_duration);
                    if let Ok(sink) = sink_clone.lock() {
                        sink.set_volume(i as f32 * volume_step);
                    }
                }
                // Ensure volume is exactly 1.0 at the end
                if let Ok(sink) = sink_clone.lock() {
                    sink.set_volume(1.0);
                }
            });

            self.levels = AudioLevels::new();

            self.scid = aac_result.scid;
        }

        for frame in &aac_result.frames {
            self.feed_au(&frame);
        }
    }

    pub fn feed_au(&mut self, au_data: &[u8]) {
        match self.decoder.decode(&au_data) {
            Ok(r) => {
                self.sink.lock().unwrap().append(SamplesBuffer::new(
                    r.channels as u16,
                    r.sample_rate as u32,
                    r.samples,
                ));

                self.levels.feed(r.channels, &r.samples);

                let (l, r) = calc_rms(&r.samples, r.channels as usize);

                if let Err(e) = self.tx.send(AudioEvent::LevelsUpdated(self.levels.clone())) {
                    log::warn!("Could not send AudioEvent update: {:?}", e);
                }
            }
            Err(e) => {
                log::error!("DEC: {}", e);
                return;
            }
        }
    }
}

unsafe impl Send for AudioDecoder {}
