use std::fmt::{self, format};

use log::{debug, error, info, warn};

#[derive(Debug)]
pub struct AudioDecoderError(pub String);

impl fmt::Display for AudioDecoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AudioDecoderError: {}", self.0)
    }
}
#[derive(Debug)]
pub struct AudioDecoder {
    // TODO: just dummy data for now
    scid: u8,
}

impl AudioDecoder {
    pub fn new(scid: u8)-> Self {
        Self { scid }
    }
    pub fn feed(&mut self, data: &[u8])-> Result<(), AudioDecoderError> {



        if (data.len() % 120) != 0 {
            return Err(AudioDecoderError(format!("invalid frame data length {:?}", data.len())));
        }

        // debug!("AC: feeding {} bytes", data.len());

        Ok(())
    }
}