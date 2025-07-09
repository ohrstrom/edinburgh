use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Entry {
    timestamp: Instant,
    value: usize,
}

#[derive(Debug, Clone)]
pub struct RateMeter {
    pub window_size: Duration,
    queue: VecDeque<Entry>,
    total_bytes: usize,
}

impl RateMeter {
    pub fn new(window_size: Duration) -> Self {
        Self {
            total_bytes: 0,
            queue: VecDeque::new(),
            window_size,
        }
    }

    pub fn entry(&mut self, value: usize) -> &mut Self {
        self.total_bytes += value;
        self.queue.push_back(Entry {
            timestamp: Instant::now(),
            value,
        });
        self
    }

    pub fn measure(&mut self) -> usize {
        let now = Instant::now();

        while self.queue.len() > 1 {
            let oldest = self.queue.front().unwrap();
            let next = self.queue.get(1).unwrap();
            let span = now.duration_since(next.timestamp);

            if span >= self.window_size {
                self.total_bytes -= oldest.value;
                self.queue.pop_front();
            } else {
                break;
            }
        }

        if self.queue.is_empty() {
            return 0;
        }

        let span = now.duration_since(self.queue.front().unwrap().timestamp);

        if span < self.window_size {
            return 0;
        }

        let elapsed_secs = self.window_size.as_secs_f64();
        (self.total_bytes as f64 / elapsed_secs).round() as usize
    }
}

impl Default for RateMeter {
    fn default() -> Self {
        Self::new(Duration::from_secs(1))
    }
}
