use image::{Pixel, Rgba, RgbaImage};

use serde::{Deserialize, Serialize};

/// A pixel row in an image to compare against.
#[derive(Clone, Debug)]
struct Segment {
    /// The position of this row in the image.
    position: (u32, u32),
    /// The row to compare against.
    row: RgbaImage,
}

/// A pattern that can be compared against an image.
///
/// This compars individual rows.
#[derive(Clone, Debug)]
pub struct Pattern {
    dimensions: (u32, u32),
    segments: Vec<Segment>,
}

impl Pattern {
    pub fn from_path<P>(path: P) -> Result<Pattern, crate::PatternError>
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
        let layout = img.sample_layout();
        if !layout.is_normal(image::flat::NormalForm::ImagePacked)
            || !layout.is_normal(image::flat::NormalForm::RowMajorPacked)
        {
            panic!("images should be packed and row major");
        }

        let mut segments = vec![];
        for (y, row) in img.rows().enumerate() {
            let mut r = vec![];
            let mut position: Option<(u32, u32)> = None;
            let end_of_row = row.len();
            for (x, pixel) in row.enumerate() {
                let opaque = pixel.channels()[3] == 0xff;
                if opaque {
                    if position.is_none() {
                        position = Some((x as u32, y as u32));
                    }
                    r.extend(pixel.channels());
                }
                if !opaque || (end_of_row - 1 == x) {
                    if let Some(position) = position.take() {
                        // println!("r: {r:?}");
                        let row = RgbaImage::from_vec(r.len() as u32 / 4, 1, r).unwrap();
                        r = vec![];
                        segments.push(Segment { position, row });
                    }
                }
            }
        }

        // Sort the segments by length
        segments.sort_by(|b, a| a.row.width().partial_cmp(&b.row.width()).unwrap());

        Self {
            dimensions,
            segments,
        }
    }

    #[cfg(test)]
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
            // println!("end; {end:?}");
            // let (start, end) = (start.unwrap() + layout.channels as usize, end.unwrap() + layout.channels as usize);
            let (start, end) = (start.unwrap(), end.unwrap() + layout.channels as usize);
            let image_slice = &data[start..end];
            let segment_slice = segment.row.as_raw().as_slice();
            // println!("image_slice: {image_slice:?} {}", image_slice.len());
            // println!("segment_slice: {segment_slice:?}, {}", segment_slice.len());
            if image_slice != segment_slice {
                // println!("No match");
                return false;
            }
        }

        true
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct PatternName(pub String);

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PatternInfo {
    /// Display string in the ui.
    pub name: PatternName,

    /// Optional description to elaborate.
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PatternMetadata {
    pub name: Option<PatternName>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PatternEntry {
    pub info: PatternInfo,
    pub path: std::path::PathBuf,
}

impl PatternEntry {
    pub fn load_pattern(&self) -> Result<Pattern, crate::PatternError> {
        Ok(Pattern::from_path(&self.path)?)
    }
}

pub fn load_patterns_directory(
    path: &std::path::Path,
) -> Result<Vec<PatternEntry>, crate::PatternError> {
    use std::collections::HashSet;

    let paths = std::fs::read_dir(path)?
        .map(|v| v.ok())
        .flatten()
        .collect::<Vec<_>>();

    let mut patterns = vec![];

    for direntry in paths {
        let path = direntry.path();
        if let Some(extension) = path.extension() {
            if extension == "png" {
                let name = path.file_name().unwrap();
                // println!("Loading: path.path(): {:?}", path.path());
                // let pattern =  Pattern::from_path(path.path())?;
                let mut info = PatternInfo {
                    name: PatternName(name.to_owned().into_string().unwrap()),
                    description: None,
                };

                let mut info_path = path.clone();
                info_path.set_extension("toml");
                if info_path.is_file() {
                    use std::io::Read;
                    let mut file = std::fs::File::open(info_path)?;
                    let mut data = String::new();
                    file.read_to_string(&mut data)?;
                    let metadata: PatternMetadata = toml::from_str(&data)?;
                    if let Some(name) = metadata.name {
                        info.name = name;
                    }
                    info.description = metadata.description.clone();
                }

                patterns.push(PatternEntry {
                    info,
                    path: path.canonicalize()?,
                });
            }
        }
    }
    Ok(patterns)
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
    #[test]
    fn test_pattern_create() {
        let mut img = RgbaImage::new(5, 2);
        let red = Rgba([255, 0, 0, 255]);
        let blue = Rgba([0, 0, 255, 255]);
        let green = Rgba([0, 255, 0, 255]);
        *img.get_pixel_mut(0, 0) = red;
        *img.get_pixel_mut(1, 0) = red;

        *img.get_pixel_mut(3, 0) = blue;
        *img.get_pixel_mut(4, 0) = blue;

        *img.get_pixel_mut(1, 1) = green;
        *img.get_pixel_mut(2, 1) = green;
        *img.get_pixel_mut(3, 1) = green;
        println!("img: {img:?} data len {}", img.as_raw().len());

        let pattern = Pattern::from_image(&img);
        println!("pattern: {pattern:?}");
        assert_eq!(pattern.dimensions.0, img.width());
        assert_eq!(pattern.dimensions.1, img.height());
        assert_eq!(pattern.segments.len(), 3);
        assert_eq!(pattern.segments[0].position, (1, 1));
        assert_eq!(pattern.segments[0].row.get_pixel(0, 0), &green);
        assert_eq!(pattern.segments[1].position, (0, 0));
        assert_eq!(pattern.segments[1].row.get_pixel(0, 0), &red);
        assert_eq!(pattern.segments[2].position, (3, 0));
        assert_eq!(pattern.segments[2].row.get_pixel(0, 0), &blue);
    }
}
