use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct FramePacer {
    next: Instant,
    previous_frame: Option<u64>,
}

impl Default for FramePacer {
    fn default() -> Self {
        Self {
            next: Instant::now(),
            previous_frame: None,
        }
    }
}

impl FramePacer {
    pub fn wait(&mut self, frame: u64, active: bool) {
        if self.previous_frame == Some(frame) {
            return;
        }
        self.previous_frame = Some(frame);
        if !active {
            self.next = Instant::now();
            return;
        }
        let now = Instant::now();
        if self.next > now {
            // At most one 16.67 ms frame. Some WGL drivers ignore VSync.
            std::thread::sleep(self.next - now);
        }
        let now = Instant::now();
        let scheduled = self.next + Duration::from_secs_f64(1.0 / 60.0);
        self.next = if scheduled > now {
            scheduled
        } else {
            now + Duration::from_secs_f64(1.0 / 60.0)
        };
    }
}

#[derive(Default)]
pub struct Performance {
    previous_frame: Option<u64>,
    previous_time: Option<f64>,
    intervals: VecDeque<f64>,
    benchmark_intervals: Vec<f64>,
    cpu_seconds: f64,
    cpu_samples: u64,
}

impl Performance {
    pub fn record(&mut self, frame: u64, seconds: f64, cpu: Option<f32>, benchmark: bool) {
        if self.previous_frame == Some(frame) {
            return;
        }
        self.previous_frame = Some(frame);
        if let Some(previous) = self.previous_time {
            let interval = seconds - previous;
            if interval > 0.0 {
                self.intervals.push_back(interval);
                if self.intervals.len() > 120 {
                    self.intervals.pop_front();
                }
                if benchmark {
                    self.benchmark_intervals.push(interval);
                }
            }
        }
        self.previous_time = Some(seconds);
        if benchmark && let Some(cpu) = cpu {
            self.cpu_seconds += f64::from(cpu);
            self.cpu_samples += 1;
        }
    }

    pub fn pause(&mut self) {
        self.previous_time = None;
    }

    pub fn fps(&self) -> f64 {
        let duration: f64 = self.intervals.iter().sum();
        if duration > 0.0 {
            self.intervals.len() as f64 / duration
        } else {
            0.0
        }
    }

    pub fn report(&self) -> String {
        let samples = &self.benchmark_intervals;
        let duration: f64 = samples.iter().sum();
        let fps = if duration > 0.0 {
            samples.len() as f64 / duration
        } else {
            0.0
        };
        let mut sorted = samples.clone();
        sorted.sort_by(f64::total_cmp);
        let p95 = sorted
            .get((sorted.len() as f64 * 0.95).floor() as usize)
            .copied()
            .unwrap_or_default()
            * 1000.0;
        let cpu_ms = self.cpu_seconds / self.cpu_samples.max(1) as f64 * 1000.0;
        format!(
            "Target FPS: 60\nObserved FPS: {fps:.2}\nFrames measured: {}\nMeasurement seconds: {duration:.3}\nP95 frame interval ms: {p95:.3}\nMean UI computation wall ms: {cpu_ms:.3}\nVSync: enabled\nIndependent frame limiter: enabled\n",
            samples.len()
        )
    }
}
