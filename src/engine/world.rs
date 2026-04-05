/// The number of f32 channels packed into every pixel (x, y) of the world.
///
/// Layout (matches wight-world):
///   [0]       energy        – how much energy this pixel has
///   [1]       age           – ticks alive
///   [2]       drain_acc     – accumulated energy drain
///   [4..18]   weights       – 14 weight channels; [16]=speed abs(w), [17]=spin w
pub const CHANNELS: usize = 19;

/// The world tensor: a flat, row-major [width × height × CHANNELS] buffer of f32.
///
/// Index a pixel at (x, y) with `world.idx(x, y)` which gives you the slice
/// start; the full channel slice is `data[i .. i + CHANNELS]`.
///
/// This is the single contiguous blob we will hand to the GPU as a storage buffer.
pub struct World {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

impl World {
    /// Allocate a zeroed world of the given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0.0_f32; width * height * CHANNELS],
        }
    }

    /// Flat index of the first channel for pixel (x, y).
    ///
    /// Panics in debug builds if (x, y) is out of bounds.
    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        debug_assert!(x < self.width, "x={x} out of bounds (width={})", self.width);
        debug_assert!(
            y < self.height,
            "y={y} out of bounds (height={})",
            self.height
        );
        (y * self.width + x) * CHANNELS
    }

    /// Read-only channel slice for pixel (x, y).
    #[inline]
    pub fn pixel(&self, x: usize, y: usize) -> &[f32] {
        let i = self.idx(x, y);
        &self.data[i..i + CHANNELS]
    }

    /// Mutable channel slice for pixel (x, y).
    #[inline]
    pub fn pixel_mut(&mut self, x: usize, y: usize) -> &mut [f32] {
        let i = self.idx(x, y);
        &mut self.data[i..i + CHANNELS]
    }

    /// Total number of f32 values in the buffer (= width × height × CHANNELS).
    #[inline]
    pub fn buffer_len(&self) -> usize {
        self.data.len()
    }
}
