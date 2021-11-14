use cortex_m_semihosting::hprintln;
use micromath::F32Ext;

const PI_X2: f32 = 6.283185307179586;

use core::marker::Copy;
#[derive(Clone, Copy)]
pub struct Interpolator {
    start: (f32, f32),
    end: (f32, f32),
    interpolation_len: f32,

    diff_1: (f32, f32),
    diff_2: (f32, f32),

    radius: f32,
    start_angle: f32,
    central_angle: f32,
    circle_origin: (f32, f32),

    method: Interpolation,
}

#[derive(Clone, Copy)]
pub enum Interpolation {
    Linear,
    Circular(f32, f32, CircularInterpolationDir),
    NoInterpolation,
}

#[derive(Clone, Copy)]
pub enum CircularInterpolationDir {
    Clockwise,
    CounterClockwise,
}

impl Interpolator {
    pub fn new(start: (i32, i32), end: (i32, i32), method: Interpolation) -> Self {
        match method {
            Interpolation::Linear => Self::setup_linear_interpolation(start, end, method),
            Interpolation::Circular(_, _, _) => {
                Self::setup_circular_interpolaion(start, end, method)
            }
            Interpolation::NoInterpolation => Self::setup_no_interpolation(start, end),
        }
    }

    #[inline]
    pub fn get_interpolation_len(&self) -> u32 {
        self.interpolation_len as u32
    }

    #[inline]
    pub fn get_interpolation_at(&self, idx: u32) -> (i32, i32) {
        match self.method {
            Interpolation::Linear => self.calc_interpolation_linear(idx),
            Interpolation::Circular(i, j, dir) => self.calc_interpolation_circular(idx, i, j, dir),
            Interpolation::NoInterpolation => self.calc_interpolation_none(),
        }
    }

    #[inline]
    fn calc_interpolation_none(&self) -> (i32, i32) {
        (self.end.0 as i32, self.end.1 as i32)
    }

    #[inline]
    fn calc_interpolation_linear(&self, idx: u32) -> (i32, i32) {
        if self.interpolation_len == 0.0 {
            return (self.end.0 as i32, self.end.1 as i32);
        }

        let fraction = (idx as f32) / self.interpolation_len;

        let x = self.start.0 + (self.diff_1.0 * fraction);
        let y = self.start.1 + (self.diff_1.1 * fraction);

        (x.floor() as i32, y.floor() as i32)
    }

    #[inline]
    fn calc_interpolation_circular(
        &self,
        idx: u32,
        i: f32,
        j: f32,
        dir: CircularInterpolationDir,
    ) -> (i32, i32) {
        let fraction = idx as f32 / self.interpolation_len;
        let offset_angle = fraction * self.central_angle;

        let angle;

        match dir {
            CircularInterpolationDir::Clockwise => angle = self.start_angle - offset_angle,
            CircularInterpolationDir::CounterClockwise => angle = self.start_angle + offset_angle,
        }

        let x = self.circle_origin.0 + (self.radius * angle.cos());
        let y = self.circle_origin.1 + (self.radius * angle.sin());

        (x.round() as i32, y.round() as i32)
    }

    fn setup_linear_interpolation(
        start: (i32, i32),
        end: (i32, i32),
        method: Interpolation,
    ) -> Self {
        let interpolation_len = if (start.0 - end.0).abs() > ((start.1 - end.1).abs()) {
            (start.0 - end.0).abs() as f32
        } else {
            (start.1 - end.1).abs() as f32
        };

        let start = (start.0 as f32, start.1 as f32);
        let end = (end.0 as f32, end.1 as f32);

        let x_diff = end.0 - start.0;
        let y_diff = end.1 - start.1;

        let diff_1 = (x_diff, y_diff);

        Self {
            start,
            end,
            interpolation_len,
            diff_1,
            method,

            diff_2: (0.0, 0.0),
            start_angle: 0.0,
            central_angle: 0.0,
            circle_origin: (0.0, 0.0),
            radius: 0.0,
        }
    }

    fn setup_circular_interpolaion(
        start: (i32, i32),
        end: (i32, i32),
        method: Interpolation,
    ) -> Self {
        let (i, j, dir) = match method {
            Interpolation::Circular(i, j, dir) => (i, j, dir),
            _ => panic!("Called circular interpolation setup without circular interpolation enum."),
        };

        let start = (start.0 as f32, start.1 as f32);
        let end = (end.0 as f32, end.1 as f32);

        let circle_origin = (start.0 + i, start.1 + j);

        let start_diff = (start.0 - circle_origin.0, start.1 - circle_origin.1);
        let end_diff = (end.0 - circle_origin.0, end.1 - circle_origin.1);

        //FIXME: calculate radius without involving squares as this might overflow
        let radius = ((start_diff.0 * start_diff.0) + (start_diff.1 * start_diff.1)).sqrt();

        let start_angle = start_diff.1.atan2(start_diff.0);
        let end_angle = end_diff.1.atan2(end_diff.0);

        let is_clockwise = if let CircularInterpolationDir::Clockwise = dir {
            true
        } else {
            false
        };

        let mut central_angle = (start_angle - end_angle).abs();
        if !is_clockwise {
            central_angle = PI_X2 - central_angle
        } //TODO: I have no idea how this works...
        if central_angle == 0.0 {
            central_angle = PI_X2;
        }

        let circumference = PI_X2 * radius * (central_angle / PI_X2);
        let circumference = circumference.round();

        Self {
            start,
            end,
            interpolation_len: circumference,
            diff_1: start_diff,
            diff_2: end_diff,
            radius,
            start_angle,
            central_angle,
            circle_origin,
            method,
        }
    }

    fn setup_no_interpolation(start: (i32, i32), end: (i32, i32)) -> Self {
        let start = (start.0 as f32, start.1 as f32);
        let end = (end.0 as f32, end.1 as f32);
        Self {
            start,
            end,
            interpolation_len: 1.0,
            diff_1: (0.0, 0.0),
            diff_2: (0.0, 0.0),
            radius: 0.0,
            start_angle: 0.0,
            central_angle: 0.0,
            circle_origin: (0.0, 0.0),
            method: Interpolation::NoInterpolation,
        }
    }

    pub fn interpolation_method(&self) -> Interpolation {
        self.method
    }
}
