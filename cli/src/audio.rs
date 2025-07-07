
use derivative::Derivative;
use faad2::{version, Decoder};
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use shared::edi::msc::{AACPResult, AudioFormat};
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
}

impl AudioDecoder {
    pub fn version() -> &'static str {
        version().0
    }

    pub fn new(scid: u8, initial_audio_format: AudioFormat) -> Self {
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

    fn fade(vol: f32, duration_ms: u64) {
        // NOTE: implement generic fade logic here
    }

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
                    if let Ok(mut sink) = sink_clone.lock() {
                        sink.set_volume(i as f32 * volume_step);
                    }
                }
                // Ensure volume is exactly 1.0 at the end
                if let Ok(mut sink) = sink_clone.lock() {
                    sink.set_volume(1.0);
                }
            });

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
            }
            Err(e) => {
                log::error!("DEC: {}", e);
                return;
            }
        }
    }
}

unsafe impl Send for AudioDecoder {}
