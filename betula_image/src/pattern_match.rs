use image::{Pixel, RgbaImage};

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
        P: AsRef<std::path::Path> + Copy,
    {
        let img = image::open(path)
            .map_err(|e| format!("failed to open {}: {e:?}", path.as_ref().display()))?;
        let img = img
            .as_rgba8()
            .ok_or("image could not be converted to rgba8")?;
        Ok(Self::from_image(img))
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

#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Hash, Eq, Ord)]
#[serde(transparent)]
pub struct PatternName(pub String);

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, PartialEq, Hash, Eq, Ord)]
pub struct PatternInfo {
    /// Display string in the ui.
    pub name: PatternName,

    /// Optional description to elaborate.
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, PartialEq)]
pub struct PatternMetadata {
    /// The pattern name associated to this pattern.
    pub name: Option<PatternName>,
    /// A description of this pattern.
    pub description: Option<String>,
    /// An identifier for the original file the pattern was created from.
    pub original: Option<String>,
}
impl PatternMetadata {
    pub fn save(&self, path: &std::path::Path) -> Result<(), crate::PatternError> {
        let toml_string = toml::to_string(&self)?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct PatternEntry {
    pub info: PatternInfo,
    pub path: std::path::PathBuf,

    pub hierarchy: Vec<String>,
}

impl PatternEntry {
    pub fn load_pattern(&self) -> Result<Pattern, crate::PatternError> {
        Pattern::from_path(&self.path)
    }
}

pub fn load_patterns_directory(
    path: &std::path::Path,
) -> Result<Vec<PatternEntry>, crate::PatternError> {
    let mut patterns = vec![];
    let mut stack: Vec<(Vec<String>, std::path::PathBuf)> = std::fs::read_dir(path)
        .map_err(|e| format!("failed to open {}: {e:?}", path.display()))?
        .filter_map(|v| v.ok())
        .map(|v| (vec![], v.path()))
        .collect::<Vec<_>>();

    while let Some((hierarchy, path)) = stack.pop() {
        // println!("{hierarchy:?}: {path:?}");
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "png" {
                    let name = path.file_name().unwrap();
                    // println!("Loading: path.path(): {:?}", path.path());
                    // let pattern =  Pattern::from_path(path.path())?;
                    let mut info = PatternInfo {
                        name: PatternName(name.to_owned().into_string().unwrap()),
                        description: None,
                    };

                    let mut info_path = path.to_owned();
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
                        info.description.clone_from(&metadata.description);
                    }

                    patterns.push(PatternEntry {
                        info,
                        path: path.canonicalize()?,
                        hierarchy,
                    });
                }
            }
        } else if path.is_dir() {
            let this_dirname = path
                .file_name()
                .ok_or("no basename")?
                .to_str()
                .ok_or("no valid string")?
                .to_string();
            let new_hierarchy: Vec<String> = hierarchy
                .iter()
                .chain(std::iter::once(&this_dirname))
                .cloned()
                .collect();
            let entries = path.read_dir()?;
            for e in entries {
                stack.push((new_hierarchy.clone(), e?.path()));
            }
        }
    }

    patterns.sort_by(|a, b| a.partial_cmp(b).unwrap());

    Ok(patterns)
}

/// Maps a enum value to a pattern name.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct EnumPatternName<T> {
    value: T,
    pattern: PatternName,
}

/// Relates an enum value to an actual pattern.
#[derive(Clone)]
pub struct EnumPattern<T: std::fmt::Debug> {
    value: T,
    pattern: Pattern,
}
impl<T: std::fmt::Debug> std::fmt::Debug for EnumPattern<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "{:?}", self.value)
    }
}

/// Helper to match enums using patterns.
///
///
/// Common use case:
/// ```
/// # use serde::{Deserialize, Serialize};
/// # use betula_image::pattern_match::{EnumMatcher, EnumPatternName, PatternEntry};
/// # use betula_image::PatternError;
/// #[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Deserialize, Serialize)]
/// enum Foo {
///     Bar,
///     Buz
/// }
/// #[derive(Deserialize, Serialize)]
/// struct MatchConfig {
///     foo_patterns: Vec<EnumPatternName<Foo>>,
/// }
/// #[derive(Clone, Debug)]
/// pub struct ImageMatcher {
///     foo_matcher: EnumMatcher<Foo>,
/// }
/// impl ImageMatcher {
///     pub fn new(config: MatchConfig, patterns: &[PatternEntry]) -> Result<ImageMatcher, PatternError> {
///         Ok(Self{
///             foo_matcher: EnumMatcher::new(&config.foo_patterns, patterns)?,
///         })
///     }
/// }
/// ```

#[derive(Debug, Clone)]
pub struct EnumMatcher<T: std::fmt::Debug + Copy + std::cmp::PartialEq<T>> {
    matchers: Vec<EnumPattern<T>>,
}

impl<T: std::fmt::Debug + Copy + std::cmp::PartialEq<T>> EnumMatcher<T> {
    /// Local helper to find the appropriate patterns from the list.
    fn find_pattern(
        patterns: &[PatternEntry],
        name: &PatternName,
    ) -> Result<Pattern, crate::PatternError> {
        let pattern_entry = patterns
            .iter()
            .find(|z| z.info.name == *name)
            .ok_or(format!("could not find pattern {name:?}"))?;
        pattern_entry.load_pattern()
    }

    /// Instantiate a new enum match, using the provided match entries and a collection of patterns to select from.
    pub fn new(
        match_entries: &[EnumPatternName<T>],
        patterns: &[PatternEntry],
    ) -> Result<EnumMatcher<T>, crate::PatternError> {
        let mut matchers = vec![];
        for entry in match_entries.iter() {
            let pattern = Self::find_pattern(patterns, &entry.pattern)?;
            matchers.push(EnumPattern {
                value: entry.value,
                pattern,
            });
        }
        Ok(EnumMatcher { matchers })
    }

    /// Test a specific enum against the image.
    pub fn test(&self, img: &image::RgbaImage, test_for: T) -> bool {
        if let Some(pattern) = self.matchers.iter().find(|v| v.value == test_for) {
            if pattern.pattern.matches_exact(img) {
                return true;
            }
        }

        false
    }

    /// Search the patterns in order to see if one matches, optionally checking 'prior' first.
    pub fn search(&self, img: &image::RgbaImage, prior: Option<T>) -> Option<T> {
        if let Some(prior_label) = &prior {
            if let Some(pattern) = self.matchers.iter().find(|v| v.value == *prior_label) {
                if pattern.pattern.matches_exact(img) {
                    return Some(pattern.value);
                }
            }
        }
        for p in self.matchers.iter() {
            if p.pattern.matches_exact(img) {
                return Some(p.value);
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use image::Rgba;

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
