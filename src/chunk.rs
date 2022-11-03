/// Split a slice into overlapping chunks where each chunk is of size `size + overlap`.
pub(crate) struct OverlappingChunks<'a, T> {
    slice: &'a [T],
    cursor: usize,
    size: usize,
    overlap: usize,
}

impl<'a, T> OverlappingChunks<'a, T> {
    pub(crate) fn new(slice: &'a [T], size: usize, overlap: usize) -> Self {
        Self { slice, cursor: 0, size, overlap }
    }
}

impl<'a, T> Iterator for OverlappingChunks<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.cursor;
        if offset < self.slice.len() {
            self.cursor += self.size;
            Some(&self.slice[offset..self.slice.len().min(offset + self.size + self.overlap)])
        } else {
            None
        }
    }
}
