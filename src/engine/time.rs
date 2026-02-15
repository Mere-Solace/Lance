use std::time::Instant;

pub struct FrameTimer {
    last: Instant,
    pub dt: f32,
}

impl FrameTimer {
    pub fn new() -> Self {
        Self {
            last: Instant::now(),
            dt: 0.0,
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;
    }
}
