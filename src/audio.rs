use crate::utils::{calc_crc_fire_code, calc_crc16_ccitt};
use log::{debug, error, info, warn};
use std::fmt::{self, format};
use std::io::Cursor;

use rodio::{OutputStream, Sink, buffer::SamplesBuffer};

use derivative::Derivative;
// use redlux::Decoder;
use crate::dec::{Decoder, Transport};


#[derive(Debug)]
pub struct AudioDecoderError(pub String);
pub struct AudioFormatError(pub String);

impl fmt::Display for AudioDecoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AudioDecoderError: {}", self.0)
    }
}

impl fmt::Display for AudioFormatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AudioFormatError: {}", self.0)
    }
}

#[derive(Debug)]
pub struct AudioFormat {
    is_sbr: bool,
    is_ps: bool,
    //
    codec: String,
    samplerate: u8,
    bitrate: usize,
    //
    au_count: usize,
}

impl AudioFormat {
    pub fn from_bytes(sf: &[u8], sf_len: usize) -> Result<Self, AudioFormatError> {
        // if data.len() != 1 {
        //     return Err(AudioFormatError(format!(
        //         "invalid AudioFormat data length {:?}",
        //         data.len()
        //     )));
        // }
        if sf[3] == 0x00 && sf[4] == 0x00 {
            warn!("AudioDecoder: AU start values are zero! Aborting format processing.");
            return Err(AudioFormatError("AU start values are zero!".to_string()));
        }

        let h = sf[2];

        debug!("from bytes: {:?}", h);

        let dac_rate = (h & 0x40) != 0;
        let is_sbr = (h & 0x20) != 0;
        let is_ps = (h & 0x08) != 0;

        let codec = if is_sbr {
            if is_ps {
                "HE-AAC v2"
            } else {
                "HE-AAC"
            }
        } else {
            "AAC-LC"
        }
        .to_string();

        let samplerate = if dac_rate { 48 } else { 32 };
        let bitrate = sf_len / 120 * 8;

        let au_count = if samplerate == 48 {
            if is_sbr {
                3
            } else {
                6
            }
        } else {
            if is_sbr {
                2
            } else {
                4
            }
        };

        Ok(Self {
            is_sbr,
            is_ps,
            codec,
            samplerate,
            bitrate,
            au_count,
        })
    }
}

#[derive(Derivative)]
#[derivative(Debug)] // Enables Debug derivation
pub struct AudioDecoder {
    // TODO: just dummy data for now
    scid: u8,
    //
    f_len: usize,
    f_count: usize,
    f_sync: usize,
    //
    sf_len: usize,
    sf_raw: Vec<u8>,
    sf_buff: Vec<u8>,
    //
    au_count: usize,
    au_start: Vec<usize>,
    //
    audio_format: Option<AudioFormat>,
    //
    // #[derivative(Debug = "ignore")]
    decoder: Decoder,
    // output
    #[derivative(Debug = "ignore")]
    _stream: OutputStream,
    #[derivative(Debug = "ignore")]
    sink: Sink,
}

impl AudioDecoder {
    pub fn new(scid: u8) -> Self {

        let mut decoder = Decoder::new(Transport::Raw);

        match decoder.set_min_output_channels(2) {
            Ok(_) => {
                debug!("DEC: set min output channels");
            }
            Err(e) => {
                error!("DEC: set min output channels error: {}", e);
            }
        }
        match decoder.set_max_output_channels(2) {
            Ok(_) => {
                debug!("DEC: set max output channels");
            }
            Err(e) => {
                error!("DEC: set max output channels error: {}", e);
            }
        }


        // SEE: http://wiki.multimedia.cx/index.php?title=MPEG-4_Audio
        let config = vec![0x13, 0x14, 0x56, 0xE5, 0x98]; // extracted from dablin


        match decoder.config_raw(&config) {
            Ok(_) => {
                debug!("DEC: config raw");
            }
            Err(e) => {
                error!("DEC: config raw error: {}", e);
            }
        }


        let (stream, handle) = OutputStream::try_default().expect("Error creating output stream");
        let sink = Sink::try_new(&handle).expect("Error creating sink");        

        // decoder.config_raw(&config).unwrap();


        Self {
            scid,
            //
            f_len: 0,
            f_count: 0,
            f_sync: 0,
            //
            sf_len: 0,
            sf_raw: Vec::new(),
            sf_buff: Vec::new(),
            //
            au_count: 0,
            au_start: vec![0; 7],
            //
            audio_format: None,
            //
            decoder: decoder,
            //
            _stream: stream,
            sink: sink,
        }
    }
    pub fn feed(&mut self, data: &[u8], f_len: usize) -> Result<(), AudioDecoderError> {
        if self.f_len != 0 {
            if self.f_len != f_len {
                return Err(AudioDecoderError(format!(
                    "frame length mismatch: {} != {}",
                    f_len, self.f_len
                )));
            }
        } else {
            if f_len < 10 {
                return Err(AudioDecoderError(format!(
                    "invalid frame data length {:?}",
                    f_len
                )));
            }

            if (5 * f_len) % 120 != 0 {
                return Err(AudioDecoderError(format!(
                    "uperframe len of len {} not divisible by 120 length",
                    f_len
                )));
            }

            debug!("INIT buffer with frame length: {}", f_len);

            self.f_len = f_len;
            self.sf_len = 5 * f_len;
            self.sf_raw.resize(self.sf_len, 0);
            self.sf_buff.resize(self.sf_len, 0);
        }

        if self.f_count == 5 {
            self.sf_raw.copy_within(self.f_len.., 0);
        } else {
            self.f_count += 1;
        }

        // debug!("AF: f-len: {} | sf-len: {} | f-count: {}", self.f_len, self.sf_len, self.f_count);

        // NOTE: erhm. no idea if this is the right approach
        self.sf_raw.splice(
            ((self.f_count - 1) * self.f_len)..((self.f_count) * self.f_len),
            data.iter().copied(),
        );

        if self.f_count < 5 {
            // NOTE: return and wait for further frames / buffering
            return Ok(());
        }

        // copy buffer
        self.sf_buff.copy_from_slice(&self.sf_raw[0..self.sf_len]);

        // debug!("AC: feeding {} bytes", data.len());

        if !self.check_sync() {
            if self.f_sync == 0 {
                info!("AD: SF sync START {} frames", self.f_sync);
            }
            self.f_sync += 1;
            return Ok(());
        }

        if self.f_sync > 0 {
            info!("AD: SF sync OK after {} frames", self.f_sync);
            self.f_sync = 0;
        }

        if self.audio_format.is_none() && self.sf_buff.len() >= 11 {
            match AudioFormat::from_bytes(&self.sf_buff, self.sf_len) {
                Ok(af) => {
                    self.audio_format = Some(af);
                    info!("AD: Audio format: {:?}", self.audio_format);
                }
                Err(e) => {
                    error!("AD: Audio format error: {}", e);
                }
            }
        }

        // decode the frames? really?
        for i in 0..self.au_count {
            
            // debug!("decode AU {}", i);

            // NOTE: check if this is correct
            let au_data = &self.sf_buff[self.au_start[i]..self.au_start[i + 1]];
            let au_len = self.au_start[i + 1] - self.au_start[i];

            let au_crc_stored = ((au_data[au_len - 2] as u16) << 8) | au_data[au_len - 1] as u16;
            let au_crc_calced = calc_crc16_ccitt(&au_data[0..au_len - 2]);

            // debug!("CRC {:04X} <> {:04X}", au_crc_stored, au_crc_calced);

            if au_crc_stored != au_crc_calced {
                warn!("AD: AU CRC mismatch!");
                continue;
            }

            // NOTE: send to aac+ decoder
            //       au_data / (au_len - 2)



            // self.decode_au(&au_data);

            // self.decode_au(au_data.to_vec());


            // try with:
            let payload = &au_data[0..au_len - 2];
            self.decode_au(payload.to_vec());

            // debug!("decode AU {} - len {}", i, au_len);
        }

        // end...
        self.f_count = 0;

        Ok(())
    }

    fn check_sync(&mut self) -> bool {
        let crc_stored = u16::from_be_bytes([self.sf_buff[0], self.sf_buff[1]]);
        let crc_calculated = calc_crc_fire_code(&self.sf_buff[2..11]);

        // debug!("crc: {:04X} : {:04X}", crc_stored, crc_calculated);

        if crc_stored != crc_calculated {
            return false;
        }

        // abort processiung if no audio format is set
        if self.audio_format.is_none() {
            debug!("AD: no audio format yet");
            return true;
        }

        // NOTE: is this how it shoud be done??
        let sf_format = self.audio_format.as_ref().unwrap();

        // set / update values for current subframe
        self.au_count = sf_format.au_count;

        // NOTE: following parts are taken "blindly" from dablin... 
        //       have yet to understand this better

        self.au_start[0] = if sf_format.samplerate == 48 {
            if sf_format.is_sbr {
                6
            } else {
                11
            }
        } else {
            if sf_format.is_sbr {
                5
            } else {
                8
            }
        };

        self.au_start[self.au_count] = self.sf_len / 120 * 110;

        self.au_start[1] = ((self.sf_buff[3] as usize) << 4) | ((self.sf_buff[4] >> 4) as usize);

        if self.au_count >= 3 {
            self.au_start[2] = (((self.sf_buff[4] & 0x0F) as usize) << 8) | (self.sf_buff[5] as usize);
        }

        if self.au_count >= 4 {
            self.au_start[3] = ((self.sf_buff[6] as usize) << 4) | ((self.sf_buff[7] >> 4) as usize);
        }

        if self.au_count == 6 {
            self.au_start[4] = (((self.sf_buff[7] & 0x0F) as usize) << 8) | (self.sf_buff[8] as usize);
            self.au_start[5] = ((self.sf_buff[9] as usize) << 4) | ((self.sf_buff[10] >> 4) as usize);
        }

        for i in 0..self.au_count {
            if self.au_start[i] >= self.au_start[i + 1] {
                warn!("AD: AU start values are invalid!");
            }
        }


        // debug!("AF: au-start: {:?}", self.au_start);


        return true;
    }
    fn decode_au(&mut self, au_data: Vec<u8>) {

        let mut pcm = vec![0i16; 4096];

        match self.decoder.fill(&au_data) {
            Ok(filled) => {

                // debug!("ENC: filled: {} : {}", au_data.len(), filled);

                match self.decoder.decode_frame(&mut pcm) {
                    Ok(_) => {
                        // debug!("DEC: decoded: {:?}", pcm.len());
                    }
                    Err(e) => {
                        error!("DEC: {}", e);
                    }
                }

                
                let decoded_frame_size = self.decoder.decoded_frame_size();
                let stream_info = self.decoder.stream_info();

                // debug!("DEC: info: {:#?}", stream_info);

                println!("DEC: {:#?}", stream_info);

                pcm.resize(decoded_frame_size, 0);


                // debug!("PCM: {:?}", pcm);
            }
            Err(e) => {
                error!("DEC: fill error: {}", e);
            }
        }

        let channels = 2;
        let sample_rate = if let Some(ref af) = self.audio_format {
            if af.samplerate == 48 { 48000 } else { 32000 }
        } else {
            48000
        };

        let source = SamplesBuffer::new(channels as u16, sample_rate, pcm);

        self.sink.append(source);

    }
}
