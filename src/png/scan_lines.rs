use super::PngData;

#[derive(Debug, Clone)]
/// An iterator over the scan lines of a PNG image
pub struct ScanLines<'a> {
    iter: ScanLineRanges,
    /// A reference to the PNG image being iterated upon
    raw_data: &'a [u8],
}

impl<'a> ScanLines<'a> {
    pub fn new(png: &'a PngData) -> Self {
        Self {
            iter: ScanLineRanges::new(png),
            raw_data: &png.raw_data,
        }
    }
}

impl<'a> Iterator for ScanLines<'a> {
    type Item = ScanLine<'a>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(len, pass)| {
            let (data, rest) = self.raw_data.split_at(len);
            self.raw_data = rest;
            let (&filter, data) = data.split_first().unwrap();
            ScanLine { filter, data, pass }
        })
    }
}

#[derive(Debug)]
/// An iterator over the scan lines of a PNG image
pub struct ScanLinesMut<'a> {
    iter: ScanLineRanges,
    /// A reference to the PNG image being iterated upon
    raw_data: Option<&'a mut [u8]>,
}

impl<'a> ScanLinesMut<'a> {
    pub fn new(png: &'a mut PngData) -> Self {
        Self {
            iter: ScanLineRanges::new(png),
            raw_data: Some(&mut png.raw_data),
        }
    }
}

impl<'a> Iterator for ScanLinesMut<'a> {
    type Item = ScanLineMut<'a>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(len, pass)| {
            let tmp = self.raw_data.take().unwrap();
            let (data, rest) = tmp.split_at_mut(len);
            self.raw_data = Some(rest);
            let (&mut filter, data) = data.split_first_mut().unwrap();
            ScanLineMut { filter, data, pass }
        })
    }
}

#[derive(Debug, Clone)]
/// An iterator over the scan line locations of a PNG image
struct ScanLineRanges {
    /// Current pass number, and 0-indexed row within the pass
    pass: Option<(u8, u32)>,
    bits_per_pixel: u8,
    width: u32,
    height: u32,
    left: usize,
}

impl ScanLineRanges {
    pub fn new(png: &PngData) -> Self {
        Self {
            bits_per_pixel: png.ihdr_data.bit_depth.as_u8() * png.channels_per_pixel(),
            width: png.ihdr_data.width,
            height: png.ihdr_data.height,
            left: png.raw_data.len(),
            pass: if png.ihdr_data.interlaced == 1 {
                Some((1, 0))
            } else {
                None
            },
        }
    }
}

impl Iterator for ScanLineRanges {
    type Item = (usize, Option<u8>);
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        let (pixels_per_line, current_pass) = if let Some(ref mut pass) = self.pass {
            // Scanlines for interlaced PNG files
            // Handle edge cases for images smaller than 5 pixels in either direction
            if self.width < 5 && pass.0 == 2 {
                pass.0 = 3;
                pass.1 = 4;
            }
            // Intentionally keep these separate so that they can be applied one after another
            if self.height < 5 && pass.0 == 3 {
                pass.0 = 4;
                pass.1 = 0;
            }
            let (pixels_factor, y_steps) = match pass {
                (1, _) | (2, _) => (8, 8),
                (3, _) => (4, 8),
                (4, _) => (4, 4),
                (5, _) => (2, 4),
                (6, _) => (2, 2),
                (7, _) => (1, 2),
                _ => unreachable!(),
            };
            let mut pixels_per_line = self.width / pixels_factor as u32;
            // Determine whether to add pixels if there is a final, incomplete 8x8 block
            let gap = self.width % pixels_factor;
            match pass.0 {
                1 | 3 | 5 if gap > 0 => {
                    pixels_per_line += 1;
                }
                2 if gap >= 5 => {
                    pixels_per_line += 1;
                }
                4 if gap >= 3 => {
                    pixels_per_line += 1;
                }
                6 if gap >= 2 => {
                    pixels_per_line += 1;
                }
                _ => (),
            };
            let current_pass = Some(pass.0);
            if pass.1 + y_steps >= self.height {
                pass.0 += 1;
                pass.1 = match pass.0 {
                    3 => 4,
                    5 => 2,
                    7 => 1,
                    _ => 0,
                };
            } else {
                pass.1 += y_steps;
            }
            (pixels_per_line, current_pass)
        } else {
            // Standard, non-interlaced PNG scanlines
            (self.width, None)
        };
        let bits_per_line = pixels_per_line * u32::from(self.bits_per_pixel);
        let bytes_per_line = ((bits_per_line + 7) / 8) as usize;
        let len = bytes_per_line + 1;
        self.left -= len;
        Some((len, current_pass))
    }
}

#[derive(Debug, Clone)]
/// A scan line in a PNG image
pub struct ScanLine<'a> {
    /// The filter type used to encode the current scan line (0-4)
    pub filter: u8,
    /// The byte data for the current scan line, encoded with the filter specified in the `filter` field
    pub data: &'a [u8],
    /// The current pass if the image is interlaced
    pub pass: Option<u8>,
}

#[derive(Debug)]
/// A scan line in a PNG image
pub struct ScanLineMut<'a> {
    /// The filter type used to encode the current scan line (0-4)
    pub filter: u8,
    /// The byte data for the current scan line, encoded with the filter specified in the `filter` field
    pub data: &'a mut [u8],
    /// The current pass if the image is interlaced
    pub pass: Option<u8>,
}
