use serde::{Deserialize, Serialize};

// Most of this file is copied from https://github.com/iwanders/displaylight_rs/blob/master/displaylight/src/lib.rs

use screen_capture::Capture;
use screen_capture::Image;
use screen_capture::Resolution;

/// Capture specification, if `match_*` is populated and matches the resolution's value it will be
/// considered to match and the capture will be setup according to the other fields.
#[derive(Debug, PartialEq, Serialize, Deserialize, Default, Copy, Clone)]
pub struct CaptureSpecification {
    /// The resolution's width to match to.
    pub match_width: Option<u32>,
    /// The resolution's height to match to.
    pub match_height: Option<u32>,

    #[serde(default)]
    /// The x offset to apply for this specification.
    pub x: u32,
    /// The y offset to apply for this specification.
    #[serde(default)]
    pub y: u32,

    /// The width to apply for this specification, set to the resolutions' width - x if zero.
    #[serde(default)]
    pub width: u32,
    /// The height to apply for this specification, set to the resolutions' height - y if zero.
    #[serde(default)]
    pub height: u32,

    /// The display to set the capture setup to.
    #[serde(default)]
    pub display: u32,
}

pub fn get_config(width: u32, height: u32, specs: &[CaptureSpecification]) -> CaptureSpecification {
    for spec in specs.iter() {
        let mut matches = true;
        if let Some(match_width) = spec.match_width {
            matches &= match_width == width;
        }
        if let Some(match_height) = spec.match_height {
            matches &= match_height == height;
        }
        if !matches {
            continue;
        }

        // We found the best match, copy this and populate it as best we can.
        let mut populated: CaptureSpecification = *spec;
        populated.width = if populated.width == 0 {
            width - populated.x
        } else {
            populated.width
        };
        populated.height = if populated.height == 0 {
            height - populated.y
        } else {
            populated.height
        };
        return populated;
    }

    // No capture match found... well, return some sane default then.
    CaptureSpecification {
        width,
        height,
        ..Default::default()
    }
}

/// Configuration struct, specifying all the configurable properties of the displaylight struct..
#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Config {
    /// A list of capture specifications, the first one to match will be used.
    pub capture: Vec<CaptureSpecification>,
}

pub struct CaptureGrabber {
    config: Config,
    grabber: Box<dyn Capture>,
    cached_resolution: Option<Resolution>,
}

impl CaptureGrabber {
    pub fn new(config: Config) -> CaptureGrabber {
        CaptureGrabber {
            config,
            grabber: screen_capture::get_capture(),
            cached_resolution: None,
        }
    }

    pub fn update_resolution(&mut self) {
        // First, check if the resolution of the desktop environment has changed, if so, act.
        let current_resolution = self.grabber.get_resolution();

        if self.cached_resolution.is_none()
            || *self.cached_resolution.as_ref().unwrap() != current_resolution
        {
            let width = current_resolution.width;
            let height = current_resolution.height;

            // Resolution has changed, figure out the best match in our configurations and
            // prepare the capture accordingly.
            let config = get_config(width, height, &self.config.capture);

            self.grabber.prepare_capture(
                config.display,
                config.x,
                config.y,
                config.width,
                config.height,
            );
            // Store the current resolution.
            self.cached_resolution = Some(current_resolution);
        }
    }

    pub fn capture(&mut self) -> Option<Box<dyn Image>> {
        self.update_resolution();

        // Now, we are ready to try and get the image:
        let res = self.grabber.capture_image();

        if !res {
            return None;
        }

        // Then, we can grab the actual image.
        Some(self.grabber.get_image())
    }
}

// And the stuff below is from landmark localizer

/// Wrapper such that we can implement GenericImageView for the RGB buffer.
pub struct CaptureAdaptor<'a> {
    pub width: usize,
    pub height: usize,
    pub buffer: &'a [screen_capture::RGB],
}
impl<'a> CaptureAdaptor<'a> {
    pub fn to_width_height_vec(&self) -> (u32, u32, Vec<u8>) {
        (
            self.width as u32,
            self.height as u32,
            self.buffer
                .iter()
                .flat_map(|rgb| {
                    let mut res = image::Rgba::<u8>::from([0; 4]);
                    res.0[0] = rgb.r;
                    res.0[1] = rgb.g;
                    res.0[2] = rgb.b;
                    res.0[3] = 0;
                    res.0
                })
                .collect(),
        )
    }
    pub fn to_width_height_vec2(&self) -> (u32, u32, Vec<u8>) {
        let mut data = Vec::<u8>::with_capacity(4 * self.width * self.height);
        for y in 0..self.height {
            let start = y * self.width;
            for x in 0..self.width {
                let p = start + x;
                data.push(self.buffer[p].r);
                data.push(self.buffer[p].g);
                data.push(self.buffer[p].b);
                data.push(0);
            }
        }
        (self.width as u32, self.height as u32, data)
    }

    pub fn to_width_height_vec3(&self) -> (u32, u32, Vec<u8>) {
        let start = self.buffer.as_ptr();
        // This is provably safe.
        let data = unsafe {
            let len = self.buffer.len();
            let ratio = std::mem::size_of::<screen_capture::RGB>() / std::mem::size_of::<u8>();
            let byte_ptr = std::mem::transmute::<*const screen_capture::RGB, *const u8>(start);
            let slice = std::slice::from_raw_parts(byte_ptr, len * ratio);
            slice.to_vec()
        };

        (self.width as u32, self.height as u32, data)
    }
}
impl<'a> image::GenericImageView for CaptureAdaptor<'a> {
    type Pixel = image::Rgba<u8>;
    fn dimensions(&self) -> (u32, u32) {
        (self.width as u32, self.height as u32)
    }
    fn width(&self) -> u32 {
        self.width as u32
    }
    fn height(&self) -> u32 {
        self.height as u32
    }

    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        let rgb = self.buffer[y as usize * self.width + x as usize];
        let mut res = image::Rgba::<u8>::from([0; 4]);
        res.0[0] = rgb.r;
        res.0[1] = rgb.g;
        res.0[2] = rgb.b;
        res.0[3] = 0;
        res
    }
}

pub trait CaptureAdapted {
    fn as_adapted(&self) -> CaptureAdaptor;
}

impl CaptureAdapted for Box<dyn screen_capture::Image> {
    fn as_adapted(&self) -> CaptureAdaptor {
        CaptureAdaptor {
            width: self.get_width() as usize,
            height: self.get_height() as usize,
            buffer: self.get_data().unwrap(),
        }
    }
}
