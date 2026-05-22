pub struct AlignedBuffer {
    storage: Vec<u8>,
    offset: usize,
    len: usize,
}

impl AlignedBuffer {
    pub fn new(len: usize, alignment: usize) -> Self {
        let storage = vec![0u8; len + alignment];
        let ptr = storage.as_ptr() as usize;
        let misalignment = ptr % alignment;
        let offset = if misalignment == 0 {
            0
        } else {
            alignment - misalignment
        };
        Self {
            storage,
            offset,
            len,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.storage[self.offset..self.offset + self.len]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.storage[self.offset..self.offset + self.len]
    }

    pub fn fill_pattern(&mut self, value: u8) {
        self.as_mut_slice().fill(value);
    }
}
