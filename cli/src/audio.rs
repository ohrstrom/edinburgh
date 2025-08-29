use cpal::traits::HostTrait;
use derive_more::Debug;
use faad2::{version, Decoder};
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamBuilder, Sink};
use shared::dab::msc::{AacpResult, AudioFormat};
use std::io::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub enum AudioEvent {
    LevelsUpdated(AudioLevels),
}

#[derive(Debug, Clone)]
pub struct AudioLevels {
    pub peak: (f32, f32),
    pub peak_clamped: (f32, f32),
    pub rms: (f32, f32),

    pub peak_smooth: (f32, f32),
    pub rms_smooth: (f32, f32),

    #[debug(skip)]
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
        let top_n = 64;

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

        let peak_l =
            peaks_l.iter().take(top_n).copied().sum::<f32>() / peaks_l.len().min(top_n) as f32;
        let peak_r =
            peaks_r.iter().take(top_n).copied().sum::<f32>() / peaks_r.len().min(top_n) as f32;

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

        // log::debug!("{:?}", self);
    }
}

#[derive(Debug)]
pub struct AudioDecoder {
    scid: u8,
    asc: Vec<u8>,
    audio_format: AudioFormat,
    #[debug(skip)]
    decoder: Decoder,
    #[debug(skip)]
    _stream: OutputStream,
    #[debug(skip)]
    sink: Arc<Mutex<Sink>>,
    #[debug(skip)]
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
        #[allow(unused_variables)] use_jack: bool,
        initial_audio_format: AudioFormat,
        tx: UnboundedSender<AudioEvent>,
    ) -> Self {
        let asc = initial_audio_format.asc.clone();
        let decoder = Decoder::new(&asc).expect("Failed to create initial decoder");

        let host: cpal::Host = {
            #[cfg(all(feature = "jack", target_os = "linux"))]
            if use_jack {
                cpal::host_from_id(cpal::HostId::Jack).expect("JACK host not available")
            } else {
                cpal::default_host()
            }
            #[cfg(not(all(feature = "jack", target_os = "linux")))]
            {
                cpal::default_host()
            }
        };

        log::debug!("available audio backends: {:?}", cpal::available_hosts());
        log::debug!("selected audio backend: {:?}", host.id());

        let device = host
            .default_output_device()
            .expect("Unable to get default device");
        let stream_handle = OutputStreamBuilder::from_device(device)
            .and_then(|x| x.open_stream())
            .expect("Error creating output stream");
        let sink = Arc::new(Mutex::new(Sink::connect_new(stream_handle.mixer())));

        Self {
            scid,
            asc,
            audio_format: initial_audio_format,
            decoder,
            _stream: stream_handle,
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
                // Err(Error::new(ErrorKind::Other, "Decoder error"))
                Err(std::io::Error::other("Decoder error"))
            }
        }
    }

    /*
    fn fade(vol: f32, duration_ms: u64) {
        // implement generic fade logic
    }
    */

    pub fn feed(&mut self, aac_result: &AacpResult) {
        if let Some(new_audio_format) = &aac_result.audio_format {
            if new_audio_format != &self.audio_format && self.reconfigure(new_audio_format).is_err()
            {
                log::warn!("Decoder reconfiguration failed, skipping audio data");
                return;
            }
        }

        if aac_result.scid != self.scid {
            log::info!("Changed SCID: {} > {}", self.scid, aac_result.scid);

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
                // ensure volume is exactly 1.0 at the end
                if let Ok(sink) = sink_clone.lock() {
                    sink.set_volume(1.0);
                }
            });

            self.levels = AudioLevels::new();

            self.scid = aac_result.scid;
        }

        for frame in &aac_result.frames {
            self.feed_au(frame);
        }
    }

    pub fn feed_au(&mut self, au_data: &[u8]) {
        match self.decoder.decode(au_data) {
            Ok(r) => {
                self.sink.lock().unwrap().append(SamplesBuffer::new(
                    r.channels as u16,
                    r.sample_rate as u32,
                    r.samples,
                ));

                self.levels.feed(r.channels, r.samples);

                if let Err(e) = self.tx.send(AudioEvent::LevelsUpdated(self.levels.clone())) {
                    log::warn!("Could not send AudioEvent update: {:?}", e);
                }
            }
            Err(e) => {
                log::error!("DEC: {}", e);
            }
        }
    }
}

unsafe impl Send for AudioDecoder {}
