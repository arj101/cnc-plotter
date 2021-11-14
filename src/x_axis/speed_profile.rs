use micromath::F32Ext;

pub struct SpeedProfile {
    profile: [Option<SpeedVector>; 32],
    profile_idx: usize,

    pub start_pos: i32,
    pub end_pos: i32,

    pub max_vel: u32,
    pub end_vel: u32,

    pub deccel_slope: f32,
}

impl SpeedProfile {
    pub fn new(start: i32, end: i32, max_v: u32, end_v: u32) -> Result<SpeedProfile, ()> {
        if max_v > 100 || end_v > max_v || end_v % 5 != 0 || start == end {
            Err(())
        } else {
            Ok(SpeedProfile {
                profile: [None; 32],
                profile_idx: 0,

                start_pos: start,
                end_pos: end,
                max_vel: max_v,
                end_vel: end_v,

                deccel_slope: -0.75,
            })
        }
    }

    fn push_vec(
        profile: &mut [Option<SpeedVector>; 32],
        profile_idx: usize,
        vec: SpeedVector,
    ) -> Result<usize, ()> {
        let new_idx = profile_idx + 1;
        if new_idx >= profile.len() {
            return Err(());
        }

        profile[profile_idx] = Some(vec);

        Ok(new_idx)
    }

    pub fn with_deccel_slope(mut self, slope: f32) -> Result<SpeedProfile, ()> {
        if slope >= 0.0 {
            Err(())
        } else {
            self.deccel_slope = slope;
            Ok(self)
        }
    }

    /// should always return `&[Some(SpeedVector)]`
    pub fn get_profile(&self) -> &[Option<SpeedVector>] {
        &self.profile[..self.profile_idx]
    }

    pub fn generate_profile(&mut self) {
        self.profile = [None; 32];
        self.profile_idx = 0;

        let mut max_vel = self.max_vel as f32;
        let mut slope = self.deccel_slope;

        let start_pos = self.start_pos as f32;
        let end_pos = self.end_pos as f32;

        if start_pos > end_pos {
            slope = slope.abs();
        }

        if start_pos < end_pos {
            if (max_vel / slope) + end_pos < start_pos {
                max_vel = (start_pos - end_pos) * slope;
            }
        } else if (max_vel / slope) + end_pos < end_pos {
            max_vel = (start_pos - end_pos) * slope;
        }

        if (max_vel / slope) + end_pos != start_pos {
            self.profile_idx = Self::push_vec(
                &mut self.profile,
                self.profile_idx,
                SpeedVector::new(self.start_pos, max_vel.floor() as u32),
            )
            .unwrap();
        }

        if max_vel % 10.0 != 0.0 {
            self.profile_idx = Self::push_vec(
                &mut self.profile,
                self.profile_idx,
                SpeedVector::new(
                    ((max_vel / slope).floor() + end_pos) as i32,
                    max_vel.floor() as u32,
                ),
            )
            .unwrap();
        }

        let mut i = (max_vel / 10.0f32).floor() as i32;
        let i_end = (self.end_vel / 10) as i32;

        while i >= i_end {
            let x = (i as f32 / slope) * 10.0;
            let y = i as f32 * 10.0;
            self.profile_idx = Self::push_vec(
                &mut self.profile,
                self.profile_idx,
                SpeedVector::new((end_pos + x).floor() as i32, y.floor() as u32),
            )
            .unwrap();
            i -= 1;
        }

        if self.end_vel != 0 {
            self.profile_idx = Self::push_vec(
                &mut self.profile,
                self.profile_idx,
                SpeedVector::new(self.end_pos, self.end_vel),
            )
            .unwrap();
        }
    }

    pub fn get_vel(&self, pos: i32) -> u32 {
        let mut vel = 0;

        if (self.start_pos < self.end_pos && pos < self.start_pos)
            || (self.start_pos > self.end_pos && pos > self.start_pos)
        {
            return self.max_vel;
        }

        for v in self.get_profile() {
            let v = v.unwrap();
            if v.pos() <= pos {
                vel = v.speed();
            }
        }

        vel
    }
}

#[derive(Clone, Copy)]
pub struct SpeedVector {
    pos: i32,   //x
    speed: u32, //y
}

impl SpeedVector {
    pub fn new(pos: i32, speed: u32) -> SpeedVector {
        SpeedVector { pos, speed }
    }

    pub fn pos(&self) -> i32 {
        self.pos
    }
    pub fn speed(&self) -> u32 {
        self.speed
    }
}
