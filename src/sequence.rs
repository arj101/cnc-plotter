use core::marker::Copy;

use heapless::{consts::U1024, Vec};
use stm32h7::stm32h743v::crc::init;

use crate::interpolator::{Interpolation, Interpolator};
use crate::pen::PenPosition;

pub struct Sequence {
    sequence_list: Vec<SequenceVector, U1024>,
    curr_sequence: usize, //array index of current sequence

    sequence_is_running: bool,
}

impl Sequence {
    pub fn new() -> Sequence {
        let mut sequence_list = Vec::new();
        sequence_list.push(SequenceVector::new(
            0,
            0,
            PenPosition::Default,
            0,
            0,
            Interpolation::Linear,
        ));

        Sequence {
            sequence_list,
            curr_sequence: 0,

            sequence_is_running: false,
        }
    }

    #[inline]
    pub fn add_pos(
        &mut self,
        x: i32,
        y: i32,
        pen: PenPosition,
        method: Interpolation,
    ) -> Result<(), ()> {
        let (prev_x, prev_y) = if self.sequence_list.len() == 0 {
            (x, y)
        } else {
            let prev = self.last_pos();
            (prev.end_x(), prev.end_y())
        };
        if let Err(_) = self
            .sequence_list
            .push(SequenceVector::new(x, y, pen, prev_x, prev_y, method))
        {
            Err(())
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn curr_pos(&self) -> SequenceVector {
        self.sequence_list[self.curr_sequence]
    }

    #[inline]
    pub fn sequence_len(&self) -> usize {
        self.sequence_list.len()
    }

    #[inline]
    pub fn curr_sqv(&self) -> SequenceVector {
        self.sequence_list[self.curr_sequence]
    }

    #[inline]
    pub fn advance(&mut self) -> Option<(i32, i32)> {
        let new_curr_sq = self.curr_sequence + 1;

        if let Some(sqv) = self.sequence_list.get(new_curr_sq) {
            self.curr_sequence = new_curr_sq;

            Some((sqv.end_x(), sqv.end_y()))
        } else {
            None
        }
    }

    pub fn skip_to(&mut self, idx: usize) -> Option<&SequenceVector> {
        let new_curr_sq = idx;

        if let Some(sqv) = self.sequence_list.get(new_curr_sq) {
            self.curr_sequence = new_curr_sq;

            Some(&sqv)
        } else {
            None
        }
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        self.sequence_is_running
    }

    pub fn start_sequence(&mut self) {
        self.sequence_is_running = true;
    }

    pub fn stop_sequence(&mut self) {
        self.sequence_is_running = false;
    }

    #[inline]
    pub fn is_starting_sequence(&self) -> bool {
        self.curr_sequence == 0
    }

    #[inline]
    pub fn has_free_space(&self) -> bool {
        self.sequence_list.len() < 1024
    }

    /// panics if `sequence_list` is of length 0
    pub fn last_pos(&self) -> SequenceVector {
        self.sequence_list[self.sequence_list.len() - 1]
    }

    pub fn clear_sequence(&mut self, initial_pos: (i32, i32)) {
        let last_pos = self.sequence_list[self.curr_sequence];
        self.sequence_list.clear();
        self.curr_sequence = 0;
        self.add_pos(
            last_pos.end_x(),
            last_pos.end_y(),
            PenPosition::Default,
            last_pos.interpolator.interpolation_method(),
        );
    }
}

/// struct for storing each printer headpositions
#[derive(Clone, Copy)]
pub struct SequenceVector {
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    pen: PenPosition,
    pub interpolator: Interpolator,
}

impl SequenceVector {
    pub fn new(
        end_x: i32,
        end_y: i32,
        pen: PenPosition,
        start_x: i32,
        start_y: i32,
        method: Interpolation,
    ) -> SequenceVector {
        let interpolator = Interpolator::new((start_x, start_y), (end_x, end_y), method);
        SequenceVector {
            start_x,
            start_y,
            end_x,
            end_y,
            pen,
            interpolator,
        }
    }

    #[inline]
    pub fn end_x(&self) -> i32 {
        self.end_x as i32 //casting for easier comparison with opto pos
    }

    #[inline]
    pub fn end_y(&self) -> i32 {
        self.end_y as i32
    }

    #[inline]
    pub fn start(&self) -> (i32, i32) {
        (self.start_x, self.start_y)
    }

    #[inline]
    pub fn end(&self) -> (i32, i32) {
        (self.end_x, self.end_y)
    }

    pub fn pen(&self) -> PenPosition {
        self.pen
    }
}
