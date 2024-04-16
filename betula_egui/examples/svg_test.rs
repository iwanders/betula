// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// use eframe::egui;

// use epaint;
// pub struct PathShape {
// pub points: Vec<Pos2>,
// pub closed: bool,
// pub fill: Color32,
// pub stroke: Stroke,
// }
use egui::{epaint, Color32, Stroke};

fn foo() -> Result<Vec<egui::Pos2>, Box<dyn std::error::Error>> {
    let mut res = vec![];
    let svg_path = "m 50.648808,238.03571 -24.190476,23.43453 46.113096,6.42559 c -6.06106,-10.26228 36.409312,-32.55192 -21.92262,-29.86012 z";
    let tokens = svg_path.split(" ").collect::<Vec<_>>();

    let mut cursor: egui::Pos2 = Default::default();

    let parse_coord = |index: usize| -> Result<egui::Pos2, Box<dyn std::error::Error>> {
        let pair = tokens[index];
        let mut it = pair.split(",");
        let left = it.next().ok_or("missing x coordinate")?;
        let right = it.next().ok_or("missing y coordinate")?;
        Ok(egui::Pos2 {
            x: left.parse()?,
            y: right.parse()?,
        })
    };
    let pop_coordinate = |index: &mut usize| -> Result<egui::Pos2, Box<dyn std::error::Error>> {
        let v = parse_coord(*index)?;
        *index = *index + 1;
        Ok(v)
    };

    let mut index = 0;
    while index < tokens.len() {
        let instruction = tokens[index];
        index += 1;
        // println!("instruction: {instruction}");
        match instruction {
            "m" => {
                // https://www.w3.org/TR/SVG2/paths.html#PathDataMovetoCommands
                // Move to, relative line.
                cursor += pop_coordinate(&mut index)?.to_vec2();
                res.push(cursor);
                while tokens[index].len() != 1 {
                    cursor += pop_coordinate(&mut index)?.to_vec2();
                    res.push(cursor);
                }
            }
            "c" => {
                // https://www.w3.org/TR/SVG2/paths.html#PathDataCubicBezierCommands
                // relative cubic bezier.
                let current = cursor;
                let c1 = cursor + pop_coordinate(&mut index)?.to_vec2();
                let c2 = cursor + pop_coordinate(&mut index)?.to_vec2();
                let end = cursor + pop_coordinate(&mut index)?.to_vec2();
                let this_bezier = epaint::CubicBezierShape {
                    points: [current, c1, c2, end],
                    closed: false,
                    fill: Color32::PLACEHOLDER,
                    stroke: Stroke::NONE,
                };
                res.extend(this_bezier.flatten(Some(1.0)));
                cursor = end;
            }
            "z" => {
                res.push(*res.first().ok_or("no point to close")?);
            }
            _ => panic!("Unhandled instruction {instruction}"),
        }
    }
    Ok(res)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // fn main(){
    let r = foo()?;
    println!("{r:#?}");
    // Ok(())
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    // Our application state:
    let mut name = "Arthur".to_owned();
    let mut age = 42;

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                age += 1;
            }
            ui.label(format!("Hello '{name}', age {age}"));

            let points = foo().expect("failed to parse svg");

            let desired_size = ui.spacing().interact_size.y * egui::vec2(100.0, 100.0);
            let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
            if ui.is_rect_visible(rect) {
                let visuals = ui.style().noninteractive();
                let rect = rect.expand(visuals.expansion);
                // ui.painter().rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
                let shape = epaint::PathShape {
                    points: points
                        .iter()
                        .cloned()
                        .map(|v| v + egui::vec2(0.0, -100.0))
                        .collect(),
                    closed: true,
                    fill: Color32::RED,
                    stroke: Stroke::NONE,
                };
                ui.painter().add(shape);
            }
        });
    });

    Ok(())
}
