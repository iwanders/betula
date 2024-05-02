use betula_image::pattern_match::load_patterns_directory;
use betula_image::PatternError;
fn main() -> Result<(), PatternError> {
    let args: Vec<String> = std::env::args().collect();
    let pattern_library = args.get(1).ok_or("no argument to library provided")?;
    let pattern_path = std::path::PathBuf::from(&pattern_library);
    let patterns = load_patterns_directory(&pattern_path)?;
    println!("Patterns: {patterns:#?}");
    for pattern in patterns.iter() {
        let r = pattern.load_pattern();
        match r {
            Ok(_) => println!("{:?}: {:?}: ok", pattern.info.name, pattern.path),
            Err(e) => println!("{:?}: {:?}: {}", pattern.info.name, pattern.path, e),
        }
    }
    Ok(())
}
