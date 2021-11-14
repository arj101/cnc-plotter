use crate::interpolator::{CircularInterpolationDir, Interpolation};
use crate::pen::PenPosition;
use crate::sequence::Sequence;
use crate::sequence::SequenceVector;

use micromath::F32Ext;
pub struct SequenceWrapper {
    unit_length_x: f32,
    unit_length_y: f32,
    pub sequence: Sequence,
    pen_pos: PenPosition,
    home_pos: (i32, i32),
}

impl SequenceWrapper {
    /// default home position is (500, 0)
    pub fn new() -> Self {
        let home_pos = (0, 0);
        Self {
            // unit_length: 0.021,
            unit_length_y: 0.0105,
            unit_length_x: 0.042,
            sequence: Sequence::new(),
            pen_pos: PenPosition::Default,
            home_pos,
        }
    }

    #[inline]
    pub fn pen_pos(&mut self, angle: f32) {
        let angle = angle.clamp(0.0, 255.0).ceil() as u8;
        self.pen_pos = PenPosition::Angle(angle);
    }

    #[inline]
    pub fn set_home(&mut self, x: f32, y: f32) {
        let x = self.mm_to_unit_x(x).round() as i32;
        let y = self.mm_to_unit_y(y).round() as i32;

        self.home_pos = (x, y);
    }

    #[inline]
    pub fn pos(&mut self, x: f32, y: f32) {
        let x = self.mm_to_unit_x(x).round() as i32;
        let y = self.mm_to_unit_y(y).round() as i32;
        let (x, y) = (x + self.home_pos.0, y + self.home_pos.1);
        self.sequence
            .add_pos(x, y, self.pen_pos, Interpolation::Linear);
    }

    #[inline]
    pub fn pos_rapid(&mut self, x: f32, y: f32) {
        let x = self.mm_to_unit_x(x).round() as i32;
        let y = self.mm_to_unit_y(y).round() as i32;
        let (x, y) = (x + self.home_pos.0, y + self.home_pos.1);
        self.sequence
            .add_pos(x, y, self.pen_pos, Interpolation::NoInterpolation);
    }

    #[inline]
    pub fn pos_x(&mut self, x: f32) {
        let x = self.mm_to_unit_x(x).round() as i32 + self.home_pos.0;
        let y = self.sequence.last_pos().end_y();
        self.sequence
            .add_pos(x, y, self.pen_pos, Interpolation::Linear);
    }

    #[inline]
    pub fn pos_y(&mut self, y: f32) {
        let y = self.mm_to_unit_y(y).round() as i32 + self.home_pos.1;
        let x = self.sequence.last_pos().end_x();
        self.sequence
            .add_pos(x, y, self.pen_pos, Interpolation::Linear);
    }

    #[inline]
    pub fn pos_x_rapid(&mut self, x: f32) {
        let x = self.mm_to_unit_x(x).round() as i32 + self.home_pos.0;
        let y = self.sequence.last_pos().end_y();
        self.sequence
            .add_pos(x, y, self.pen_pos, Interpolation::NoInterpolation);
    }

    #[inline]
    pub fn pos_y_rapid(&mut self, y: f32) {
        let y = self.mm_to_unit_y(y).round() as i32 + self.home_pos.1;
        let x = self.sequence.last_pos().end_x();
        self.sequence
            .add_pos(x, y, self.pen_pos, Interpolation::NoInterpolation);
    }

    #[inline]
    pub fn arc_absolute_center(
        &mut self,
        x: f32,
        y: f32,
        i: f32,
        j: f32,
        dir: CircularInterpolationDir,
    ) {
        let x = self.mm_to_unit_x(x).round() as i32;
        let y = self.mm_to_unit_y(y).round() as i32;

        let i = self.mm_to_unit_x(i);
        let j = self.mm_to_unit_y(j);

        let (x, y) = (x + self.home_pos.0, y + self.home_pos.1);
        let (i, j) = (i + self.home_pos.0 as f32, j + self.home_pos.1 as f32);

        let last_pos = self.sequence.last_pos();
        let last_x = last_pos.end_x() as f32;
        let last_y = last_pos.end_y() as f32;
        self.sequence.add_pos(
            x,
            y,
            self.pen_pos,
            Interpolation::Circular(i - last_x, j - last_y, CircularInterpolationDir::Clockwise),
        );
    }

    #[inline]
    pub fn arc_clockwise_absolute_center(&mut self, x: f32, y: f32, i: f32, j: f32) {
        self.arc_absolute_center(x, y, i, j, CircularInterpolationDir::Clockwise)
    }

    #[inline]
    pub fn arc_counter_clockwise_absolute_center(&mut self, x: f32, y: f32, i: f32, j: f32) {
        self.arc_absolute_center(x, y, i, j, CircularInterpolationDir::CounterClockwise)
    }

    #[inline]
    pub fn arc_relative_center(
        &mut self,
        x: f32,
        y: f32,
        i: f32,
        j: f32,
        dir: CircularInterpolationDir,
    ) {
        let x = self.mm_to_unit_x(x).round() as i32;
        let y = self.mm_to_unit_y(y).round() as i32;

        let i = self.mm_to_unit_x(i);
        let j = self.mm_to_unit_y(j);

        let (x, y) = (x + self.home_pos.0, y + self.home_pos.1);
        self.sequence
            .add_pos(x, y, self.pen_pos, Interpolation::Circular(i, j, dir));
    }

    #[inline]
    pub fn arc_clockwise_relative_center(&mut self, x: f32, y: f32, i: f32, j: f32) {
        self.arc_relative_center(x, y, i, j, CircularInterpolationDir::Clockwise)
    }

    #[inline]
    pub fn arc_counter_clockwise_relative_center(&mut self, x: f32, y: f32, i: f32, j: f32) {
        self.arc_relative_center(x, y, i, j, CircularInterpolationDir::CounterClockwise)
    }

    // aliases --------------------------------------
    #[inline]
    /// alias for `arc_clockwise_relative_center`
    pub fn arc_clockwise(&mut self, x: f32, y: f32, i: f32, j: f32) {
        self.arc_clockwise_relative_center(x, y, i, j)
    }

    #[inline]
    /// alias for `arc_counter_clockwise_relative_center`
    pub fn arc_counter_clockwise(&mut self, x: f32, y: f32, i: f32, j: f32) {
        self.arc_clockwise_relative_center(x, y, i, j)
    }
    // -------------------------------------------------

    #[inline]
    pub fn mm_to_unit_x(&self, value: f32) -> f32 {
        value / self.unit_length_x
    }

    #[inline]
    pub fn mm_to_unit_y(&self, value: f32) -> f32 {
        value / self.unit_length_y
    }

    #[inline]
    pub fn start(&mut self) {
        self.sequence.start_sequence()
    }

    #[inline]
    pub fn stop(&mut self) {
        self.sequence.stop_sequence()
    }

    #[inline]
    pub fn clear(&mut self, curr_pos: (i32, i32)) {
        self.sequence.clear_sequence(curr_pos)
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        self.sequence.is_running()
    }

    #[inline]
    pub fn curr_pos(&self) -> SequenceVector {
        self.sequence.curr_pos()
    }

    #[inline]
    pub fn advance(&mut self) -> Option<(i32, i32)> {
        self.sequence.advance()
    }
}
