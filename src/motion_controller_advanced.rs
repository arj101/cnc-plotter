use crate::timestamp;
use crate::com::CommandHandler;
use crate::pen::pen_driver::PenDriver;
use crate::pen::PenPosition;
use crate::pwm::{MotorPwmX, MotorPwmY};
use crate::sequence_wrapper::SequenceWrapper;
use crate::stop_timer::StopTimer;

use crate::opto_encoder::Encoder;
use crate::speed_calc::PulseContedSpeedCalc;
use crate::{EncoderX, EncoderY};

use cortex_m_semihosting::hprintln;

use micromath::F32Ext;
use stm32h7xx_hal::delay::Delay;

use crate::global_ethernet::eth_send;
use crate::motion_controller_2::PathSlope;

use stm32h7xx_hal::prelude::_embedded_hal_blocking_delay_DelayMs;

pub struct MotionController {
    x_motor: MotorPwmX,
    y_motor: MotorPwmY,

    x_opto: PulseContedSpeedCalc<EncoderX>,
    y_opto: PulseContedSpeedCalc<EncoderY>,

    pen_driver: PenDriver,

    sequence: SequenceWrapper,
    stop_timer: StopTimer,

    int_idx: f32,

    x_pwm: f32,
    y_pwm: f32,

    last_correction_time_x: u64,
    last_correction_time_y: u64,
}

impl MotionController {
    pub fn new(
        x_motor: MotorPwmX,
        y_motor: MotorPwmY,
        encoder_x: EncoderX,
        encoder_y: EncoderY,
        pen_driver: PenDriver,
    ) -> Self {
        Self {
            x_motor,
            y_motor,
            x_opto: PulseContedSpeedCalc::new(encoder_x),
            y_opto: PulseContedSpeedCalc::new(encoder_y),
            pen_driver,
            sequence: SequenceWrapper::new(),
            stop_timer: StopTimer::new(),
            int_idx: 0.0,

            x_pwm: 0.0,
            y_pwm: 0.0,

            last_correction_time_x: timestamp(),
            last_correction_time_y: timestamp(),
        }
    }

    pub fn calibrate(&mut self, delay: &mut Delay) {
        self.x_left(80.0);
        delay.delay_ms(3000u16);
        self.x_stop();
        self.x_opto.calibrate();
    }

    #[inline]
    pub fn start_sequence(&mut self) {
        self.sequence.start();
    }

    #[inline]
    pub fn stop_sequence(&mut self) {
        self.sequence.stop();
    }

    #[inline]
    fn interpret_gcode(&mut self, code: &gcode::GCode) {
        match code.major_number() {
            0 => {
                //G00 rapid move
                if let Some(pen_pos) = code.value_for('Z') {
                    self.sequence.pen_pos(pen_pos)
                }
                match (code.value_for('X'), code.value_for('Y')) {
                    (Some(x), Some(y)) => self.sequence.pos_rapid(x, y),
                    (Some(x), None) => self.sequence.pos_x_rapid(x),
                    (None, Some(y)) => self.sequence.pos_y_rapid(y),
                    (None, None) => (),
                }
            }
            1 => {
                //G01 linear interpolation
                if let (Some(x), Some(y)) = (code.value_for('X'), code.value_for('Y')) {
                    self.sequence.pos(x, y)
                }
            }
            _ => (),
        }
    }

    #[inline]
    pub fn tick(&mut self, cmd: &mut CommandHandler) {
        self.x_opto.tick(self.x_pwm);
        self.y_opto.tick(self.y_pwm);

        if self.sequence.sequence.has_free_space() {
            for code in cmd.get_gcode_buffer() {
                self.interpret_gcode(code);
                if !self.sequence.is_running() {
                    self.start_sequence();
                }
            }
            cmd.clear_gcode_buffer();
        }
   
        if !self.sequence.is_running()
            || self.sequence.sequence.sequence_len() <= 1
            || self.stop_timer.is_running()
        {
            self.x_stop();
            self.y_stop();
            return;
        }

        let (x1, y1) = self.sequence.curr_pos().start();
        let (x1, y1) = (x1 as f32, y1 as f32);
        let (x2, y2) = self.sequence.curr_pos().end();
        let (x2, y2) = (x2 as f32, y2 as f32);
        let (cx, cy) = (self.x_opto.pos() as f32, self.y_opto.pos() as f32);
        let len = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();

        let mut dir_x = (x2 - x1) / (x2 - x1).abs();
        let mut dir_y = (y2 - y1) / (y2 - y1).abs();

        if dir_x.is_nan() {
            dir_x = 0.0;
        }

        if dir_y.is_nan() {
            dir_y = 0.0;
        }

        if (((cx >= x2 && dir_x > 0.0) || (cx <= x2 && dir_x < 0.0) || dir_x == 0.0) && ((cy >= y2 && dir_y > 0.0) || (cy <= y2 && dir_y < 0.0) || dir_y == 0.0)) || (dir_x == 0.0 && dir_y == 0.0){
            if let None = self.sequence.advance() {
                self.x_stop();
                self.y_stop();
                return;
            }
            let pen_pos = self.sequence.curr_pos().pen();
            self.pen_driver.move_pen(pen_pos);
            self.int_idx = 0.0;
            return;
        }

        let (di_x, di_y) = (x2 - x1, y2 - y1);
        let mut t = self.int_idx / len;
        let (de_x, mut de_y) = (x1 + di_x * t, y1 + di_y * t);

        // eth_send!("di_x: {}, di_y: {}\n", di_x, di_y);
        // eth_send!("de_x: {}, de_y: {}, x: {}, y: {}\n", de_x, de_y, self.x_opto.pos(), self.y_opto.pos());

        if self.x_opto.pos() as f32 == de_x && self.y_opto.pos() as f32 == de_y {
            self.int_idx += 1.0;
            self.move_x(dir_x, 36.0);
            self.move_y(dir_y, 18.0);
        } else if ((self.y_opto.pos() as f32) < de_y && dir_y > 0.0) || ((self.y_opto.pos() as f32) > de_y && dir_y < 0.0) {
            self.x_stop();
            self.move_y(dir_y, 18.0);
        } else if ((self.x_opto.pos() as f32) < de_x && dir_x > 0.0) || ((self.x_opto.pos() as f32) > de_x && dir_x < 0.0) {
            self.y_stop();
            self.move_x(dir_x, 36.0);
        } 
        else {
            if ((self.x_opto.pos() as f32) > de_x && dir_x > 0.0) || ((self.x_opto.pos() as f32) < de_x && dir_x < 0.0) {
                t = (cx-x1) / di_x;
                self.int_idx = t * len;
                de_y = y1 + di_y * t;
                // eth_send!("here2 t: {} cx: {}, x1: {}, int: {}, di_x: {}\n", t, cx, x1, self.int_idx, di_x);
            }

            if ((self.y_opto.pos() as f32) > de_y && dir_y > 0.0) || ((self.y_opto.pos() as f32) < de_y && dir_y < 0.0) {
                t = (cy-y1) / di_y;
                self.int_idx = t * len;
            }
        }
    }

    fn move_x(&mut self, dir: f32, pwm: f32) {
        if dir > 0.0 {
            self.x_right(pwm);
        } else if dir < 0.0 {
            self.x_left(pwm);
        } else {
            self.x_stop();
        }
    }

    fn move_y(&mut self, dir: f32, pwm: f32) {
        if dir > 0.0 {
            self.y_up(pwm);
        } else if dir < 0.0 {
            self.y_down(pwm);
        } else {
            self.y_stop();
        }
    }

    fn x_left(&mut self, pwm: f32) {
        self.x_motor.move_left(pwm);
        self.x_pwm = -pwm;
    }

    fn x_right(&mut self, pwm: f32) {
        self.x_motor.move_right(pwm);
        self.x_pwm = pwm;
    }

    fn x_stop(&mut self) {
        self.x_motor.0.active_stop();
        self.x_pwm = 0.0;
    }

    fn y_down(&mut self, pwm: f32) {
        self.y_motor.move_down(pwm);
        self.y_pwm = -pwm;
    }

    fn y_up(&mut self, pwm: f32) {
        self.y_motor.move_up(pwm);
        self.y_pwm = pwm;
    }

    fn y_stop(&mut self) {
        self.y_motor.0.active_stop();
        self.y_pwm = 0.0;
    }

    #[inline]
    pub fn curr_pos(&self) -> (i32, i32) {
        (self.x_opto.pos(), self.y_opto.pos())
    }
}
