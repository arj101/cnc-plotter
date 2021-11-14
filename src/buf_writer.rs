use core::fmt;
pub struct BufWriter<'a> {
    data: &'a mut [u8],
    next_buf_idx: usize,
}

impl<'a> BufWriter<'a> {
    pub fn new(buf: &mut [u8]) -> BufWriter {
        BufWriter {
            data: buf,
            next_buf_idx: 0,
        }
    }
    pub fn get_bytes(&self) -> &[u8] {
        &self.data[..self.next_buf_idx]
    }
    pub fn re_init_buf(&mut self, buf: &'a mut [u8]) {
        self.data = buf;
        self.next_buf_idx = 0;
    }
    pub fn clear_buf(&mut self) {
        self.next_buf_idx = 0;
    }
}

impl<'a> core::fmt::Write for BufWriter<'a> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        if self.next_buf_idx + s.len() >= self.data.len() - 1 {
            Err(fmt::Error)
        } else {
            for c in s.chars() {
                self.data[self.next_buf_idx] = c as u8;
                self.next_buf_idx += 1;
            }
            Ok(())
        }
    }
}
