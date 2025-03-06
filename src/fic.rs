use std::fmt::{self, format};

#[derive(Debug)]
pub struct FICDecoderError(pub String);

impl fmt::Display for FICDecoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FICDecoderError: {}", self.0)
    }
}
#[derive(Debug)]
pub struct FICDecoder {
    // TODO: just dummy data for now
    eid: Option<String>,
}

impl FICDecoder {
    pub fn from_bytes(data: &[u8]) -> Result<Self, FICDecoderError> {
        if (data.len() % 32) != 0 {
            return Err(FICDecoderError(format!(
                "invalid FIC data length {:?}",
                data.len()
            )));
        }

        Ok(Self { eid: None })
    }
}
