use egui::{epaint, Color32, Pos2, Stroke, Vec2};

pub struct SVGPaths {
    /// Actual viewbox for this svg, top right only, bottom left is assumed as (0.0, 0.0);
    pub viewbox: Vec2,
    /// Transform to apply to the paths.
    pub transform: Vec2,
    /// SVG path strings.
    pub paths: Vec<String>,

    /// The fill the shape, this only works for convex polygons.
    pub fill: Color32,

    /// Whether to apply a stroke to the polygon.
    pub stroke: Stroke,
}

impl SVGPaths {
    pub fn to_shapes(
        &self,
        scale: f32,
        bezier_tolerance: Option<f32>,
    ) -> Result<Vec<Vec<Pos2>>, Box<dyn std::error::Error>> {
        // Calculate the desired shapes
        let mut res = vec![];
        for svg_path in &self.paths {
            let mut this_path = vec![];
            let tokens = svg_path.split(" ").collect::<Vec<_>>();

            let mut cursor: egui::Pos2 = Default::default();

            let parse_coord = |index: usize| -> Result<egui::Pos2, Box<dyn std::error::Error>> {
                let pair = tokens[index];
                let mut it = pair.split(",");
                let left = it.next().ok_or("missing x coordinate")?;
                let right = it.next().ok_or("missing y coordinate")?;
                Ok(egui::Pos2 {
                    x: left.parse::<f32>()? * scale,
                    y: right.parse::<f32>()? * scale,
                })
            };
            let pop_coordinate =
                |index: &mut usize| -> Result<egui::Pos2, Box<dyn std::error::Error>> {
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
                        this_path.push(cursor);
                        while tokens[index].len() != 1 {
                            cursor += pop_coordinate(&mut index)?.to_vec2();
                            this_path.push(cursor);
                        }
                    }
                    "l" => {
                        // https://www.w3.org/TR/SVG2/paths.html#PathDataLinetoCommands
                        while tokens[index].len() != 1 {
                            cursor += pop_coordinate(&mut index)?.to_vec2();
                            this_path.push(cursor);
                        }
                    }
                    "c" => {
                        // https://www.w3.org/TR/SVG2/paths.html#PathDataCubicBezierCommands
                        // relative cubic bezier.
                        while tokens[index].len() != 1 {
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
                            this_path.extend(this_bezier.flatten(bezier_tolerance));
                            cursor = end;
                        }
                    }
                    "z" => {
                        this_path.push(*this_path.first().ok_or("no point to close")?);
                    }
                    _ => panic!("Unhandled instruction {instruction}"),
                }
            }
            for c in this_path.iter_mut() {
                *c = *c + self.transform * scale;
            }
            // println!("this_path len: {}", this_path.len());
            res.push(this_path);
        }

        Ok(res)
    }

    fn calculate_scale(&self, desired_size: Vec2) -> f32 {
        let x_scaling = desired_size.x / self.viewbox.x;
        let y_scaling = desired_size.y / self.viewbox.y;
        x_scaling.min(y_scaling)
    }

    pub fn to_shapes_within(
        &self,
        desired_size: Vec2,
    ) -> Result<Vec<Vec<Pos2>>, Box<dyn std::error::Error>> {
        let scale = self.calculate_scale(desired_size);
        let shapes = self.to_shapes(scale, Some(scale * 0.1))?;
        Ok(shapes)
    }

    pub fn to_widget(&self, desired_size: Vec2) -> impl egui::Widget + '_ {
        let shapes = self.to_shapes_within(desired_size).expect("should work");

        move |ui: &mut egui::Ui| {
            let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
            let response_rect = response.rect;
            if ui.is_rect_visible(response_rect) {
                // let rect = response_rect.expand(visuals.expansion);

                for points in shapes {
                    let shape = epaint::PathShape {
                        // offset the shape with the rectangle in which we are drawing.
                        points: points
                            .iter()
                            .cloned()
                            .map(|v| v + response_rect.min.to_vec2())
                            .collect(),
                        closed: true,
                        fill: self.fill,
                        stroke: self.stroke,
                    };
                    painter.add(shape);
                }
            }

            response
        }
    }

    pub fn to_svg(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut svg: String = r#"<?xml version="1.0" standalone="no"?>"#.to_owned();
        svg.push_str(&format!(
            r#"<svg width="{w}mm" height="{h}mm"
               viewBox="0 0 {w} {h}" xmlns="http://www.w3.org/2000/svg"
               version="1.1">"#,
            w = self.viewbox.x,
            h = self.viewbox.y
        ));
        svg.push_str(&format!(
            r#"<g transform="translate({x},{y})" >"#,
            x = self.transform.x,
            y = self.transform.y
        ));
        for path in &self.paths {
            svg.push_str(&format!(
                r#" <path d="{path}" style="fill:#000000;fill-opacity:1;stroke:none" />"#
            ));
        }
        svg.push_str("</g></svg>");
        Ok(svg)
    }

    pub fn write_svg(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(path, self.to_svg()?).expect("Unable to write file");
        Ok(())
    }

    pub fn write_svg_rasterized(
        &self,
        path: &std::path::Path,
        desired_size: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let shapes = self.to_shapes_within(desired_size)?;
        let scale = self.calculate_scale(desired_size);

        let paths: Vec<String> = shapes
            .iter()
            .map(|p| {
                let mut v: String = Default::default();
                v.push_str("M "); // absolute here.
                for c in p {
                    v.push_str(&format!("{},{} ", c.x, c.y));
                }
                v.push_str("z");
                v
            })
            .collect();
        let rasterized_path = SVGPaths {
            viewbox: self.viewbox * scale,
            transform: Default::default(),
            paths,
            fill: Color32::TRANSPARENT,
            stroke: Default::default(),
        };

        std::fs::write(path, rasterized_path.to_svg()?).expect("Unable to write file");
        Ok(())
    }
}

pub fn add_name_editor(
    ui: &mut egui::Ui,
    current: &str,
    edit: &mut Option<String>,
    dest: &mut Option<String>,
) {
    if let Some(ref mut editor_string) = edit {
        let edit_box = egui::TextEdit::singleline(editor_string)
            .desired_width(0.0)
            .clip_text(false);
        let r = ui.add(edit_box);
        if r.lost_focus() {
            if current != *editor_string {
                *dest = Some(editor_string.clone());
            }
            *edit = None;
            ui.close_menu();
        }
    } else {
        let r = ui.label(format!("{}", &current));
        if r.clicked() {
            *edit = Some(current.to_owned());
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_make_widget() -> Result<(), Box<dyn std::error::Error>> {
        let svg_paths = SVGPaths{
            viewbox: egui::vec2(100.0, 100.0),
            transform: egui::vec2(0.0, -197.0),
            paths: vec![
                "m 50.648808,238.03571 c -30.680991,-5.86026 -26.209966,9.52808 -24.190476,23.43453 l 46.113096,6.42559 c -6.06106,-10.26228 36.409312,-32.55192 -21.92262,-29.86012 z".to_owned(),
                "m 55.279017,214.4122 -14.079612,13.51265 37.797618,5.48066 c 0,0 -24.190475,-19.0878 -6.898066,-11.52828 17.29241,7.55953 15.497024,-3.2128 15.308035,-4.34672 -0.188986,-1.13393 -32.127975,-3.11831 -32.127975,-3.11831 z".to_owned()
            ],
            fill: Color32::TRANSPARENT,
            stroke: Default::default(),
        };
        let desired_size = egui::vec2(10.0, 10.0);
        let _ = svg_paths.to_widget(desired_size);
        svg_paths.write_svg(&std::path::PathBuf::from("/tmp/test_svg_widget.svg"))?;
        svg_paths.write_svg_rasterized(
            &std::path::PathBuf::from("/tmp/test_svg_widget_rasterized.svg"),
            desired_size,
        )?;

        let svg_paths = SVGPaths{
            viewbox: egui::vec2(100.0, 100.0),
            transform: egui::vec2(0.0, -197.0),
            paths: vec![
                "m 43.666015,289.96634 -20.433078,-35.39113 -22.68307393,22.68307 -2.75e-6,-79.5916 68.92834368,39.79581 -30.985656,8.30258 20.433077,35.39113 z".to_owned(),
            ],
            fill: Color32::TRANSPARENT,
            stroke: Default::default(),
        };
        let desired_size = egui::vec2(10.0, 10.0);
        let _ = svg_paths.to_widget(desired_size);
        svg_paths.write_svg(&std::path::PathBuf::from("/tmp/test_svg_cursor.svg"))?;
        svg_paths.write_svg_rasterized(
            &std::path::PathBuf::from("/tmp/test_svg_cursor_rasterized.svg"),
            desired_size,
        )?;

        /*

            let desired_size = ui.spacing().interact_size.y * egui::vec2(1.0, 1.0);
            // println!("desired_size: {desired_size:?}");
            ui.add(svg_paths.to_widget(desired_size));
        */
        Ok(())
    }
}
