use betula_image::pattern_match::{PatternMetadata, PatternName};
use betula_image::PatternError;
use clap::{arg, value_parser, Arg, Command};
use image::io::Reader as ImageReader;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Clone, Copy)]
struct SegmentSpec {
    x: u32,
    y: u32,
    length: usize,
}

impl std::str::FromStr for SegmentSpec {
    type Err = PatternError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let d: Vec<&str> = s.split(',').collect();
        let x_fromstr = d
            .get(0)
            .ok_or(format!("x not provided"))?
            .parse::<u32>()
            .map_err(|_| format!("x couldn't convert to i32"))?;
        let y_fromstr = d
            .get(1)
            .ok_or(format!("y not provided"))?
            .parse::<u32>()
            .map_err(|_| format!("y couldn't convert to i32"))?;
        let len_fromstr = d
            .get(2)
            .ok_or(format!("length not provided"))?
            .parse::<usize>()
            .map_err(|_| format!("length couldn't convert to usize"))?;

        Ok(SegmentSpec {
            x: x_fromstr,
            y: y_fromstr,
            length: len_fromstr,
        })
    }
}

fn main() -> Result<(), PatternError> {
    let cmd = clap::Command::new("image_pattern")
        .bin_name("image_pattern")
        .subcommand_required(true)
         .subcommand(
            Command::new("create")
                .about("Create a new pattern.")
                .arg(arg!([image] "The image to make a pattern of.").required(true).value_parser(clap::value_parser!(std::path::PathBuf)))
                .arg(Arg::new("segments")
                      .action(clap::ArgAction::Append)
                      .value_name("SEGMENTS")
                      .help("Provide segments as x,y,len  x2,y2,len2")
                      .value_parser(value_parser!(SegmentSpec))
                      .num_args(1..).required(true))
                .arg(
                    clap::arg!(--"output-dir" <PATH>).value_parser(clap::value_parser!(std::path::PathBuf))
                    .default_value("."),
                )
                .arg(
                    clap::arg!(--"filename" <FILENAME> "Use this filename in the output directory instead of the input filename"),
                )
                .arg(
                    clap::arg!(--"name" <NAME> "Defaults to the file name of the image.").value_parser(clap::builder::NonEmptyStringValueParser::new()),
                )
                .arg(
                    clap::arg!(--"description" <DESCRIPTION> "A longer description of this pattern." ).value_parser(clap::builder::NonEmptyStringValueParser::new()),
                ),
        )
        .get_matches();

    if let Some(matches) = cmd.subcommand_matches("create") {
        let output_dir = matches.get_one::<PathBuf>("output-dir").unwrap();
        let input_image = matches.get_one::<PathBuf>("image").unwrap();
        let name_default = matches
            .get_one::<PathBuf>("image")
            .map(|v| v.file_name().clone())
            .flatten()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let name = matches.get_one::<String>("name").unwrap_or(&name_default);
        let filename = matches
            .get_one::<String>("filename")
            .unwrap_or(&name_default);
        let segments: Vec<SegmentSpec> = matches
            .get_many("segments")
            .expect("segments is required")
            .copied()
            .collect();
        let description = matches.get_one::<String>("description");
        // println!("output_dir: {output_dir:?}");
        // println!("name: {name:?}");
        // println!("segments: {segments:?}");
        // println!("description: {description:?}");
        // println!("filename: {filename:?}");

        let mut output_path = output_dir.clone();
        output_path.push(filename);
        output_path.set_extension("png");

        let metadata = PatternMetadata {
            name: Some(PatternName(name.clone())),
            description: description.cloned(),
            original: Some(name_default),
        };

        let img = ImageReader::open(input_image)?.decode()?.to_rgba8();
        let mut new_img = image::RgbaImage::new(img.width(), img.height());
        for spec in segments {
            for i in 0..spec.length {
                *new_img.get_pixel_mut(spec.x + i as u32, spec.y) =
                    *img.get_pixel(spec.x + i as u32, spec.y);
            }
        }
        new_img.save(&output_path)?;
        output_path.set_extension("toml");
        metadata.save(&output_path)?;
    }

    Ok(())
}
