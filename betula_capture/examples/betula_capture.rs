use std::env::temp_dir;
fn main() {
    use betula_capture::capture::CaptureAdapted;
    let mut grabber = screen_capture::get_capture();

    let res = grabber.get_resolution();

    println!("Capture reports resolution of: {:?}", res);
    grabber.prepare_capture(0, 1920, 0, res.width - 1920, res.height);

    let mut res = grabber.capture_image();
    while !res {
        res = grabber.capture_image();
    }

    println!("Capture tried to capture image, succes? {}", res);
    let img = grabber.get_image();
    println!("Capture writing to temp {:?}", temp_dir());
    img.write_ppm(
        temp_dir()
            .join("foo.ppm")
            .to_str()
            .expect("path must be ok"),
    )
    .unwrap();
    println!("Capture done writing");

    let buffer = img.get_data();
    if buffer.is_none() {
        panic!("image didn't provide any data");
    }

    let z = screen_capture::read_ppm(
        temp_dir()
            .join("foo.ppm")
            .to_str()
            .expect("path must be ok"),
    )
    .expect("must be good");
    z.write_ppm(
        temp_dir()
            .join("bar.ppm")
            .to_str()
            .expect("path must be ok"),
    )
    .unwrap();

    println!("Cloning image.");

    use std::time::{Duration, Instant};
    let start = Instant::now();

    let z = img.as_adapted();
    // 7ms
    let start = Instant::now();
    let (width, height, data) = z.to_width_height_vec();
    let img = image::RgbaImage::from_vec(width, height, data);
    let duration = start.elapsed();
    println!("Time elapsed in to_width_height_vec() is: {:?}", duration);

    let start = Instant::now();
    let (width, height, data) = z.to_width_height_vec2();
    let img = image::RgbaImage::from_vec(width, height, data);
    let duration = start.elapsed();
    println!("Time elapsed in to_width_height_vec2() is: {:?}", duration);

    let start = Instant::now();
    let (width, height, data) = z.to_width_height_vec3();
    let img = image::RgbaImage::from_vec(width, height, data);
    let duration = start.elapsed();
    println!("Time elapsed in to_width_height_vec3() is: {:?}", duration);

    // 10'ish
    // with fn?
    let start = Instant::now();
    use image::GenericImageView;
    let img = image::RgbaImage::from_fn(z.width(), z.height(), |x, y| z.get_pixel(x, y));
    let duration = start.elapsed();
    println!("Time elapsed in from_fn() is: {:?}", duration);

    /*

    let cloned_buffer = z.get_data().expect("expect a data buffer to be present");
    let orig_buffer = img.get_data().expect("expect a data buffer to be present");
    if cloned_buffer != orig_buffer {
        println!("{:?}\n{:?}", &cloned_buffer[0..20], &orig_buffer[0..20]);
        println!("cloned_buffer: {}", cloned_buffer.len());
        println!("orig_buffer: {}", orig_buffer.len());
        panic!("data of rasterimage not equivalent to real image");
    }

    println!("Capture writing to temp.");
    z.write_ppm(temp_dir().join("z.ppm").to_str().expect("path must be ok"))
        .unwrap();
    z.write_bmp(temp_dir().join("z.bmp").to_str().expect("path must be ok"))
        .unwrap();
    println!("Capture done writing");
    println!("First pixel: {:#?}", img.get_pixel(0, 0));
    println!(
        "last pixel: {:#?}",
        img.get_pixel(img.get_width() - 1, img.get_height() - 1)
    );

    for _i in 0..2 {
        let res = grabber.capture_image();
        println!("Capture tried to capture image, succes? {}", res);
        let img = grabber.get_image();
        println!(
            "last pixel: {:#?}",
            img.get_pixel(img.get_width() - 1, img.get_height() - 1)
        );
    }
    */
}
