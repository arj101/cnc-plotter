use crate::opto_encoder::Encoder;
use crate::timestamp;

trait U64Time {
    fn sec(self) -> u64;
    fn millis(self) -> u64;
    fn micros(self) -> u64;
}

impl U64Time for u64 {
    fn sec(self) -> Self {
        self * 100_000
    }
    fn millis(self) -> Self {
        self * 1000
    }
    fn micros(self) -> Self {
        self
    }
}

pub struct DynamicSpeedCalculator<ENC: Encoder> {
    encoder: ENC,
    speed: f32,
    sampling_interval: u64,
    last_sample_time: u64,
    last_sample_pos: i32,
}

impl<ENC: Encoder> DynamicSpeedCalculator<ENC> {
    pub fn new(encoder: ENC) -> Self {
        Self {
            encoder,
            speed: 0.0,
            sampling_interval: 100.millis(),
            last_sample_time: timestamp(),
            last_sample_pos: 0,
        }
    }

    #[inline]
    pub fn tick(&mut self) {
        if timestamp() - self.last_sample_time >= self.sampling_interval {
            let speed = (self.encoder.pos() - self.last_sample_pos) as f32
                / (timestamp() - self.last_sample_time) as f32;

            self.speed = speed;
            if speed / (self.sampling_interval as f32 / 1000_000.0) < 1.0 {
                if self.sampling_interval < 500.millis() {
                    self.sampling_interval += 10_000;
                    if self.sampling_interval > 500.millis() {
                        self.sampling_interval = 500.millis()
                    }
                }
            } else if speed / (self.sampling_interval as f32 / 1000_000.0) > 1000.0 {
                if self.sampling_interval > 1.millis() {
                    self.sampling_interval -= 1_000;
                    if self.sampling_interval < 1.millis() {
                        self.sampling_interval = 1.millis()
                    }
                }
            }

            self.last_sample_pos = self.encoder.pos();
            self.last_sample_time = timestamp();
        }
    }

    #[inline]
    pub fn speed(&self) -> f32 {
        self.speed / (self.sampling_interval as f32 / 1000_000.0)
    }

    #[inline]
    pub fn pos(&self) -> i32 {
        self.encoder.pos()
    }
}

pub struct PulseContedSpeedCalc<ENC: Encoder> {
    encoder: ENC,
    last_sample_time: u64,
    last_pos: i32,
    speed: f32,
}

impl<ENC: Encoder> PulseContedSpeedCalc<ENC> {
    pub fn new(encoder: ENC) -> Self {
        Self {
            encoder,
            last_sample_time: timestamp(),
            last_pos: 0,
            speed: 0.0,
        }
    }

    #[inline]
    pub fn tick(&mut self, pwm_speed: f32) {
        if self.encoder.pos() != self.last_pos {
            self.speed = (self.encoder.pos() - self.last_pos) as f32
                / ((timestamp() - self.last_sample_time) as f32 / 1000_000.0);
            self.last_sample_time = timestamp();
            self.last_pos = self.encoder.pos();
        } else if (timestamp() - self.last_sample_time > 100.millis() && pwm_speed <= 10.0) || timestamp() - self.last_sample_time > 200.millis() {
            self.speed = 0.0
        }
    }

    #[inline]
    pub fn speed(&self) -> f32 {
        self.speed
    }

    #[inline]
    pub fn last_sample_time(&self) -> u64 {
        self.last_sample_time
    }

    #[inline]
    pub fn pos(&self) -> i32 {
        self.encoder.pos()
    }

    #[inline]
    pub fn calibrate(&self) {
        self.encoder.calibrate();
    }
}
