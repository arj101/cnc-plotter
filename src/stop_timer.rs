use crate::timestamp;

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
