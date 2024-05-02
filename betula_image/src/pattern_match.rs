use image::{Rgba, RgbaImage};

#[derive(Clone, Debug)]
struct Segment {
    /// The position of this row in the image.
    position: (u32, u32),
    /// The row to compare against.
    row: RgbaImage,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    dimensions: (u32, u32),
    segments: Vec<Segment>,
}

impl Pattern {
    pub fn open<P>(path: P) -> Result<Pattern, crate::PatternError>
    where
        P: AsRef<std::path::Path>,
    {
        let img = image::open(path)?;
        let img = img
            .as_rgba8()
            .ok_or("image could not be converted to rgba8")?;
        Ok(Self::from_image(&img))
    }

    pub fn from_image(img: &RgbaImage) -> Pattern {
        let dimensions = (img.width(), img.height());
        let segments = vec![];
        Self {
            dimensions,
            segments,
        }
    }

    fn from_segments(width: u32, height: u32, segments: &[Segment]) -> Self {
        Self {
            dimensions: (width, height),
            segments: segments.to_vec(),
        }
    }

    /// Fast equality comparison.
    pub fn matches_exact(&self, img: &RgbaImage) -> bool {
        if img.width() != self.dimensions.0 || img.height() != self.dimensions.1 {
            return false;
        }

        let layout = img.sample_layout();
        if !layout.is_normal(image::flat::NormalForm::ImagePacked)
            || !layout.is_normal(image::flat::NormalForm::RowMajorPacked)
        {
            panic!("images should be packed and row major");
        }

        let data = img.as_raw();
        // Dimensions are identical, next up is iterating through segments and comparing bytes.
        for segment in self.segments.iter() {
            let start = layout.index(0, segment.position.0, segment.position.1);
            let end = layout.index(
                0,
                segment.position.0 + (segment.row.width() - 1),
                segment.position.1,
            );
            if start.is_none() || end.is_none() {
                panic!("start {start:?} or end {end:?} index wasn't valid");
            }
            println!("end; {end:?}");
            // let (start, end) = (start.unwrap() + layout.channels as usize, end.unwrap() + layout.channels as usize);
            let (start, end) = (start.unwrap(), end.unwrap() + layout.channels as usize);
            let image_slice = &data[start..end];
            let segment_slice = segment.row.as_raw().as_slice();
            println!("image_slice: {image_slice:?} {}", image_slice.len());
            println!("segment_slice: {segment_slice:?}, {}", segment_slice.len());
            if image_slice != segment_slice {
                println!("No match");
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_pattern_match() {
        // Test whether we include the last pixel.
        let mut img = RgbaImage::new(10, 1);
        *img.get_pixel_mut(6, 0) = Rgba([255, 0, 0, 255]);
        *img.get_pixel_mut(9, 0) = Rgba([0, 0, 255, 255]);
        println!("img: {img:?} data len {}", img.as_raw().len());

        let mut row = RgbaImage::new(4, 1);
        *row.get_pixel_mut(0, 0) = Rgba([255, 0, 0, 255]);
        *row.get_pixel_mut(3, 0) = Rgba([0, 0, 255, 255]);
        println!(
            "row: {row:?}, width: {} data len {}",
            row.width(),
            row.as_raw().len()
        );
        let pattern = Segment {
            position: (6, 0),
            row,
        };
        println!("pattern: {pattern:?}");
        let pattern = Pattern::from_segments(img.width(), img.height(), &[pattern]);
        assert!(pattern.matches_exact(&img));
    }
}
