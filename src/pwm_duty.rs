pub struct PwmDutyCycle {
    max: u16,
    duty_cycle: f32,
}

impl PwmDutyCycle {
    pub fn new(max: u16) -> PwmDutyCycle {
        PwmDutyCycle {
            max,
            duty_cycle: 0.0,
        }
    }

    /// duty cycle: 0-100
    #[inline]
    pub fn set_duty_cycle(&mut self, duty_cycle: f32) {
        if duty_cycle > 100.0 || duty_cycle < 0.0 {
            panic!("duty cycle out of range(0-100): {}", duty_cycle);
        }
        self.duty_cycle = duty_cycle / 100.0;
    }

    #[inline]
    pub fn set_duty_cycle_val(&mut self, duty_cycle: u16) {
        self.duty_cycle = duty_cycle as f32 / self.max as f32;
    }

    #[inline]
    pub fn get_duty_cycle(&self) -> f32 {
        self.duty_cycle * 100.0
    }

    #[inline]
    pub fn get_duty_cycle_val(&self) -> u16 {
        (self.duty_cycle * self.max as f32) as u16
    }
}
