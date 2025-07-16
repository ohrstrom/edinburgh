use log;
use std::fmt;

#[derive(Debug)]
pub struct FrameDecodeError(pub String);

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FrameDecodeError: {}", self.0)
    }
}

impl std::error::Error for FrameDecodeError {}

#[derive(Debug, Clone)]
struct SyncMagic {
    pattern: Vec<u8>,
    #[allow(dead_code)]
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

#[derive(Debug, Clone)]
pub struct AFFrame {
    // NOTE: it looks like we only have AF frames..
    pub data: Vec<u8>,
    pub initial_size: usize,
    pub expected_size: usize,
    sync_magic: SyncMagic,
}

impl AFFrame {
    pub fn new() -> Self {
        AFFrame {
            data: vec![0; 8],
            initial_size: 8,
            expected_size: 0,
            sync_magic: SyncMagic::new(vec![b'A', b'F'], "AF"),
        }
    }

    // scan the frame for a sync magic
    pub fn find_sync_magic(&self) -> Option<usize> {
        // maximum magic length.
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
        if d.len() == 0 {
            log::debug!("check_completed: empty frame");
            return false;
        }
        if d.len() == 8 {
            // log::debug!("check_completed: header only");
            // header only > retrieve payload len and resize frame
            let len = (d[2] as usize) << 24
                | (d[3] as usize) << 16
                | (d[4] as usize) << 8
                | (d[5] as usize);

            self.expected_size = len + 10 + 2;
            self.resize(10 + len + 2);
            // log::debug!("check_completed: resize to {}", self.data.len());
            false
        } else {
            // log::debug!("check_completed: frame {}", d.len());
            true
        }
    }

    pub fn resize(&mut self, new_size: usize) {
        self.data.resize(new_size, 0);
    }

    pub fn reset(&mut self) {
        self.resize(self.initial_size);
        self.expected_size = self.initial_size;
    }
}

impl fmt::Display for AFFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AF ({})", self.data.len())
    }
}

#[derive(Debug)]
pub struct EDIFrameExtractor {
    pub frame: AFFrame,
}

impl EDIFrameExtractor {
    pub fn new() -> Self {
        EDIFrameExtractor {
            frame: AFFrame::new(),
        }
    }
}
