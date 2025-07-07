use derivative::Derivative;
use faad2::{version, Decoder};
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};

use shared::edi::msc::{AACPResult, AudioFormat};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AudioDecoder {
    asc: Vec<u8>,
    audio_format: AudioFormat,
    #[derivative(Debug = "ignore")]
    decoder: Decoder,
    #[derivative(Debug = "ignore")]
    _stream: OutputStream,
    #[derivative(Debug = "ignore")]
    sink: Sink,
}

impl AudioDecoder {
    pub fn version() -> &'static str {
        version().0
    }
    pub fn new(audio_format: AudioFormat) -> Self {
        // let asc = vec![0x13, 0x0C, 0x56, 0xE5, 0x9D, 0x48, 0x80]; // HE-AAC v2 - 24kHz

        let asc = audio_format.asc.clone();

        let decoder = Decoder::new(&asc).unwrap();

        let (stream, handle) = OutputStream::try_default().expect("Error creating output stream");
        let sink = Sink::try_new(&handle).expect("Error creating sink");

        Self {
            asc,
            audio_format,
            decoder,
            _stream: stream, // NOTE: we need to keep the stream alive
            sink,
        }
    }
    pub fn feed(&mut self, aac_result: &AACPResult) {
        if aac_result.audio_format.as_ref() != Some(&self.audio_format) {
            log::warn!(
                "Audio format mismatch: expected {:?}, got {:?}",
                self.audio_format,
                aac_result.audio_format
            );
            return;
        }

        for frame in &aac_result.frames {
            self.feed_au(&frame);
        }
    }
    pub fn feed_au(&mut self, au_data: &[u8]) {
        match self.decoder.decode(&au_data) {
            Ok(r) => {
                self.sink.append(SamplesBuffer::new(
                    r.channels as u16,
                    r.sample_rate as u32,
                    r.samples,
                ));
            }
            Err(e) => {
                log::error!("DEC: {}", e);
                return;
            } // Err(e) => {
              //     log::error!("DEC: {} — resetting decoder", e);

              //     match Decoder::new(&self.asc) {
              //         Ok(new_decoder) => {
              //             self.decoder = new_decoder;
              //             log::warn!("Decoder reset done — will try next AU");
              //         }
              //         Err(_) => {
              //             log::error!("Decoder reset failed — stuck until next good AU!");
              //         }
              //     }

              //     return;
              // }
        }
    }
}

unsafe impl Send for AudioDecoder {}
