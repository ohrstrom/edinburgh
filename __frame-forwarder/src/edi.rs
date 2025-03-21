use std::fmt::{self, format};

use log::{debug, error, info, warn};

#[derive(Debug)]
pub struct FrameDecodeError(pub String);

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FrameDecodeError: {}", self.0)
    }
}

impl std::error::Error for FrameDecodeError {}

#[derive(Debug)]
struct SyncMagic {
    pattern: Vec<u8>,
    name: String,
}

impl SyncMagic {
    fn new(pattern: Vec<u8>, name: impl Into<String>) -> Self {
        Self {
            pattern,
            name: name.into(),
        }
    }

    fn matches(&self, data: &[u8]) -> bool {
        data.starts_with(&self.pattern)
    }
}

#[derive(Debug)]
pub struct AFFrame {
    pub data: Vec<u8>,
    pub initial_size: usize,
    sync_magic: SyncMagic,
}

impl AFFrame {
    pub fn new() -> Self {
        AFFrame {
            data: vec![0; 8],
            initial_size: 8,
            sync_magic: SyncMagic::new(vec![b'A', b'F'], "AF"),
        }
    }

    pub fn find_sync_magic(&self) -> Option<usize> {
        let magic_len = self.sync_magic.pattern.len();

        for offset in 0..=self.data.len().saturating_sub(magic_len) {
            let slice = &self.data[offset..];
            if self.sync_magic.matches(slice) {
                return Some(offset);
            }
        }
        None
    }

    pub fn check_completed(&mut self) -> bool {
        let d = &self.data;
        if d.len() == 8 {
            let len = (d[2] as usize) << 24
                | (d[3] as usize) << 16
                | (d[4] as usize) << 8
                | (d[5] as usize);
            self.resize(10 + len + 2);
            false
        } else {
            true
        }
    }

    pub fn resize(&mut self, new_size: usize) {
        self.data.resize(new_size, 0);
    }

    pub fn reset(&mut self) {
        self.resize(self.initial_size);
    }
}

impl fmt::Display for AFFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AF ({})", self.data.len())
    }
}

#[derive(Debug)]
pub struct EDISource {
    pub frame: AFFrame,
}

impl EDISource {
    pub fn new() -> Self {
        EDISource {
            frame: AFFrame::new(),
        }
    }
}
