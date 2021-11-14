use super::pwm::MotorPwm;
use super::pwm::MotorPwmX;
use super::sequence::Sequence;
use super::speed_profile::SpeedProfile;
use super::timestamp;
use crate::opto_encoder::EncoderX;
use crate::pwm::PwmPinX;
use heapless::consts::U11;
use heapless::Vec;
// use super::x_pwm::XMotorPwm;

use cortex_m_semihosting::{hprint, hprintln};
use stm32h7xx_hal::delay::Delay;
use stm32h7xx_hal::prelude::_embedded_hal_blocking_delay_DelayMs;

use crate::motion_controller_2::PathSlope;

use core::marker::Copy;
use micromath::F32Ext;

const SCALING_FACTOR: f32 = 2.0 / 4.0;

#[derive(Copy, Clone)]
enum TargetDir {
    Left,
    Right,
    Stopped,
}

pub struct XDriver {
    motor: MotorPwmX,
    pub opto: EncoderX,
    sequence: Sequence,
    // speed_profile: SpeedProfile,
    target_dir: TargetDir,

    target_pos: i32,
}

impl XDriver {
    pub fn new(motor_pwm: MotorPwm<PwmPinX>, opto: EncoderX) -> XDriver {
        let motor = MotorPwmX(motor_pwm);
        XDriver {
            motor,
            opto,
            sequence: Sequence::new(),
            target_pos: 0,
            // speed_profile: SpeedProfile::new(0, 1234, 50, 5)
            //     .unwrap()
            //     .with_deccel_slope(-0.25)
            //     .unwrap(),
            target_dir: TargetDir::Stopped,
        }
    }

    fn is_at_pos(&self, pos: i32) -> bool {
        let pos = Self::scale_pos(pos);
        self.opto.pos() == pos
    }

    pub fn start_sequence(&mut self) {
        self.sequence.start_sequence();
        // self.speed_profile.generate_profile();
        hprintln!("Starting sequence...");
        // self.motor.move_right(15.0);
    }

    pub fn stop_sequence(&mut self) {
        self.sequence.stop_sequence();
    }

    #[inline]
    pub fn move_towards(&mut self, pos: i32, slope: PathSlope) -> bool {
        let pos = Self::scale_pos(pos);

        if self.target_pos != pos {
            self.target_pos = pos;
            if self.target_pos > self.opto.pos() {
                self.target_dir = TargetDir::Right;
            } else if self.target_pos < self.opto.pos() {
                self.target_dir = TargetDir::Left;
            } else {
                self.target_dir = TargetDir::Stopped;
            }
        }

        const MIN_SPEED: f32 = 33.0; //33
        const MAX_SPEED: f32 = 55.0; //55
        const SPEED_DIFF: f32 = MAX_SPEED - MIN_SPEED;

        let curr_target_dir = if pos > self.opto.pos() {
            TargetDir::Right
        } else if pos < self.opto.pos() {
            TargetDir::Left
        } else {
            TargetDir::Stopped
        };

        let enter_error_correction_mode = match (self.target_dir, curr_target_dir) {
            (TargetDir::Left, TargetDir::Right) => true,
            (TargetDir::Right, TargetDir::Left) => true,
            _ => false,
        };

        let speed = if !enter_error_correction_mode {
            match slope {
                PathSlope::NoSlope => MIN_SPEED,
                PathSlope::Slope(slope) => {
                    let mut slope = slope;

                    if slope > 1.0 || slope < 0.0 {
                        hprintln!("[x_driver.rs] slope > 1.0 || < 0.0");
                    }

                    if slope > 1.0 {
                        slope = 1.0
                    }
                    if slope < 0.0 {
                        slope = 0.0
                    }

                    MAX_SPEED - (slope * SPEED_DIFF)
                }
            }
        } else {
            MIN_SPEED
        };

        // hprintln!("[x_driver.rs] speed: {}", speed);

        if pos > self.opto.pos() {
            self.motor.move_right(speed);
            false
        } else if pos < self.opto.pos() {
            self.motor.move_left(speed);
            false
        } else {
            true
        }
    }

    // #[inline]
    // pub fn move_fast(&mut self, pos: i32, dir: i8) {
    //     let pos = Self::scale_pos(pos);
    //     if pos > self.opto.pos() {
    //         if dir > 0 {
    //             self.motor.move_right(50.0);
    //         } else if dir < 0 {
    //             self.motor.move_left(20.0);
    //         }
    //     } else if pos < self.opto.pos() {
    //         if dir < 0 {
    //             self.motor.move_left(50.0);
    //         } else if dir < 0 {
    //             self.motor.move_right(20.0);
    //         }
    //     } else {
    //         self.motor.0.active_stop();
    //     }
    // }

    pub fn calibrate(&mut self, delay: &mut Delay) {
        hprintln!("[x_driver.rs] Starting calibration sequence...");
        self.motor.move_left(90.0);
        delay.delay_ms(3000u16);

        //self.opto.calibrate();

        //let mut motor_speed = 0.;

        //self.motor.move_right(motor_speed);

        //let mut prev_tp = timestamp() as f32;
        //let mut prev_tp2 = timestamp();
        //let mut prev_pos = 0.0f32;
        //let mut avg_speed = 0.0;

        //let mut motor_dir = true;

        //let mut speeds: Vec<f32, U11> = Vec::new();
        //while motor_speed <= 100.0 {
        //   if timestamp() as f32 - prev_tp > 1_000.0 {
        //       let speed = (self.opto.pos() as f32 - prev_pos).abs() / (timestamp() as f32 - prev_tp);
        //       prev_tp = timestamp() as f32;
        //       prev_pos = self.opto.pos() as f32;
        //      avg_speed = (avg_speed + speed) / 2.0;
        // }

        //if self.opto.pos() as f32 - prev_pos > 2000. || timestamp() - prev_tp2 > 1000_000 {
        //    speeds.push(avg_speed);
        //    avg_speed = 0.0;
        //    prev_tp2 = timestamp();
        //   motor_speed += 10.0;

        //   if self.opto.pos() > 4100 {
        //       motor_dir = false;
        //   } else if self.opto.pos() < 100 {
        //       motor_dir = true;
        //  }

        //let motor_speed = if motor_speed > 100.0 { 100.0 } else { motor_speed };
        // if motor_dir {
        //    self.motor.move_right(motor_speed);
        // } else {
        //    self.motor.move_left(motor_speed);
        //}
        //}

        //}
        self.motor.0.disable_pwm();

        //for (idx, speed) in speeds.iter().enumerate() {
        //  hprintln!("duty cycle: {}, avg. speed: {}", idx * 10, speed);
        //}

        hprintln!("[x_driver.rs] Calibration complete!");
    }

    #[inline]
    pub fn stop(&mut self) {
        self.motor.0.active_stop();
    }

    #[inline]
    pub fn scale_pos(pos: i32) -> i32 {
        let pos = pos as f32;
        (pos * SCALING_FACTOR).round() as i32
    }
}
