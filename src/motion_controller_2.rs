use crate::opto_encoder::Encoder;
use crate::timestamp;
use crate::{
    com::CommandHandler, ethernet::ethernet_wrapper::EthernetWrapper, interpolator::Interpolation,
    pen::pen_driver, pen::pen_driver::PenDriver, pen::PenPosition, sequence::Sequence,
    sequence_wrapper::SequenceWrapper, BufWriter, XDriver, YDriver,
};

use cortex_m_semihosting::hprintln;

use micromath::F32Ext;
use stm32h7xx_hal::delay::Delay;

use core::marker::Copy;

#[derive(Clone, Copy)]
pub enum PathSlope {
    Slope(f32),
    NoSlope,
}

pub struct MotionController {
    x_driver: XDriver,
    y_driver: YDriver,
    pen_driver: PenDriver,

    sequence: SequenceWrapper,
    stop_timer: StopTimer,

    int_idx: u32,
}

impl MotionController {
    pub fn new(x_driver: XDriver, y_driver: YDriver, pen_driver: PenDriver) -> Self {
        Self {
            x_driver,
            y_driver,
            pen_driver,
            sequence: SequenceWrapper::new(),
            stop_timer: StopTimer::new(),
            int_idx: 0,
        }
    }

    pub fn calibrate(&mut self, delay: &mut Delay) {
        self.x_driver.calibrate(delay);
    }

    #[inline]
    pub fn start_sequence(&mut self) {
        self.sequence.start();
        let pen_pos = self.sequence.curr_pos().pen();
        self.pen_driver.move_pen(pen_pos);
        self.stop_timer.start_timer(400);
    }

    #[inline]
    pub fn stop_sequence(&mut self) {
        self.sequence.stop();
        self.x_driver.stop();
        self.y_driver.stop();
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
            self.y_driver.stop();
            self.x_driver.stop();
            return;
        }

        let target_pos = self.sequence.curr_pos();

        let (x, y) = target_pos.interpolator.get_interpolation_at(self.int_idx);

        let slope = self.calc_slope((x, y));

        // use core::fmt::write;
        // write( buf, format_args!("# slope: {}\n\r", slope));
        // eth.send( buf);

        match (
            self.x_driver.move_towards(x, slope),
            self.y_driver.move_towards(y, slope),
        ) {
            (true, true) => {
                self.int_idx += 1;

                if self.int_idx >= target_pos.interpolator.get_interpolation_len() {
                    self.int_idx = 0;
                    if let Some(_) = self.sequence.advance() {
                        let sqv = self.sequence.curr_pos();
                        self.pen_driver.move_pen(sqv.pen());

                        let old_angle = match target_pos.pen() {
                            PenPosition::Default => pen_driver::UP_ANGLE,
                            PenPosition::Angle(a) => a,
                        };

                        let new_angle = match sqv.pen() {
                            PenPosition::Default => pen_driver::UP_ANGLE,
                            PenPosition::Angle(a) => a,
                        };

                        let angle_diff = f32::abs((new_angle - old_angle) as f32);
                        let ms_10_degress = 70.0;
                        let timer_length = ((angle_diff / 10.0) * ms_10_degress).round() as u64;

                        if timer_length > 0 {
                            self.stop_timer.start_timer(timer_length);
                        }
                    } else {
                        // self.pen_driver.move_up();
                        self.stop_sequence();
                        self.sequence.clear(self.curr_pos());
                    }
                }
            }
            (true, false) => {
                self.x_driver.stop();
            }
            (false, true) => {
                self.y_driver.stop();
            }
            (false, false) => (),
        }
    }

    #[inline]
    fn calc_slope(&self, pos2: (i32, i32)) -> PathSlope {
        let pos1 = self.curr_pos();
        let pos1 = (pos1.0 as f32, pos1.1 as f32);

        let pos2 = (pos2.0 as f32, pos2.1 as f32);

        let x_diff = pos2.0 - pos1.0;
        let y_diff = pos2.1 - pos1.1;

        if x_diff == 0.0 && y_diff == 0.0 {
            return PathSlope::NoSlope;
        }

        let mut angle = y_diff.atan2(x_diff);

        // hprintln!("pos1: {:?}, pos2: {:?}, angle: {}", pos1, pos2, angle.to_degrees());

        use core::f32::consts::PI;

        const PI_X2: f32 = PI * 2.0;
        const PI_FRAC2: f32 = PI / 2.0;
        const PI_2FRAC3_4: f32 = 2.0 * PI * (3.0 / 4.0);

        if angle < 0.0 {
            angle = PI_X2 - angle.abs();
        }

        let slope_1st_quadrant = || -> f32 { angle / PI_FRAC2 };

        let slope_2nd_quadrant = || -> f32 {
            let angle = angle - PI_FRAC2;
            1.0 - (angle / PI_FRAC2)
        };

        let slope_3rd_quadrant = || -> f32 {
            let angle = angle - PI;
            angle / PI_FRAC2
        };

        let slope_4th_quadrant = || -> f32 {
            let angle = angle - PI_2FRAC3_4;
            1.0 - (angle / PI_FRAC2)
        };

        let slope;

        if angle <= PI_FRAC2 {
            slope = slope_1st_quadrant()
        } else if angle <= PI {
            slope = slope_2nd_quadrant()
        } else if angle <= PI_2FRAC3_4 {
            slope = slope_3rd_quadrant()
        } else {
            slope = slope_4th_quadrant()
        }

        // use core::fmt::write;
        // write( buf, format_args!("# x_diff:{}, y_diff:{} \n\r# slope: {}\n\r# angle: {}\n\r", x_diff, y_diff, slope, angle));
        // eth.send( buf);

        PathSlope::Slope(slope)
    }

    #[inline]
    pub fn curr_pos(&self) -> (i32, i32) {
        (self.x_driver.opto.pos(), self.y_driver.opto.pos())
    }
}

#[derive(Clone, Copy)]
pub struct StopTimer {
    start: u64,
    end: u64,

    status: bool,
}

impl StopTimer {
    pub fn new() -> StopTimer {
        StopTimer {
            start: 0u64,
            end: 0u64,
            status: false,
        }
    }

    pub fn start_timer(&mut self, length_ms: u64) {
        self.start = timestamp();
        self.end = self.start + (length_ms * 1000);
        self.status = true;
    }

    pub fn reset_timer(&mut self) {
        self.status = false;
    }

    /// calling this function also resets the timer if it has expired
    #[inline]
    pub fn has_expired(&mut self) -> bool {
        if self.has_expired_no_rst() {
            self.status = false;
            true
        } else {
            false
        }
    }

    /// alternaitve to `has_expired`which doesnt reset the timer
    #[inline]
    pub fn has_expired_no_rst(&self) -> bool {
        self.status && timestamp() >= self.end
    }

    #[inline]
    pub fn status(&self) -> bool {
        self.status
    }

    #[inline]
    /// this doesn't reset the timer
    pub fn is_running(&mut self) -> bool {
        self.status && !self.has_expired()
    }
}
