use enigo::agent::Token;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnigoPreset {
    pub index: Vec<String>,
    pub info: EnigoPresetInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnigoPresetInfo {
    pub description: Option<String>,
    pub actions: Vec<Token>,
}

fn parse_toml_file(data: &str) -> Result<Vec<EnigoPreset>, betula_core::BetulaError> {
    let value = data.parse::<toml::Table>()?;
    //println!("Value: {value:#?}");

    let mut output = vec![];
    fn recurser(
        t: &toml::Table,
        index: &[String],
        out: &mut Vec<EnigoPreset>,
    ) -> Result<(), betula_core::BetulaError> {
        for (k, v) in t.iter() {
            if let toml::Value::Table(ref t) = v {
                let mut index = index.to_vec();
                index.push(k.to_owned());
                if t.contains_key("actions") {
                    let info: EnigoPresetInfo = toml::Value::try_into(v.clone())?;
                    out.push(EnigoPreset { index, info })
                } else {
                    recurser(t, &index, out)?;
                }
            }
        }
        Ok(())
    }

    recurser(&value, &[], &mut output)?;

    Ok(output)
}

pub fn load_preset_directory(
    path: &std::path::Path,
) -> Result<Vec<EnigoPreset>, betula_core::BetulaError> {
    let mut patterns = vec![];
    let mut stack: Vec<(Vec<String>, std::path::PathBuf)> = std::fs::read_dir(path)?
        .map(|v| v.ok())
        .flatten()
        .map(|v| (vec![], v.path()))
        .collect::<Vec<_>>();

    while let Some((hierarchy, path)) = stack.pop() {
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "toml" {
                    use std::io::Read;
                    let mut file = std::fs::File::open(path)?;
                    let mut data = String::new();
                    file.read_to_string(&mut data)?;
                    let additions = parse_toml_file(&data)?;
                    patterns.extend(additions);
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

    patterns.sort_by(|a, b| a.index.partial_cmp(&b.index).unwrap());

    Ok(patterns)
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_toml_preset() -> Result<(), betula_core::BetulaError> {
        let mut examples = std::collections::HashMap::new();
        examples.insert(
            "text",
            enigo::agent::Token::Text("Well hello there".to_owned()),
        );
        examples.insert(
            "key",
            enigo::agent::Token::Key(enigo::Key::Backspace, enigo::Direction::Press),
        );
        examples.insert(
            "button",
            enigo::agent::Token::Button(enigo::Button::Middle, enigo::Direction::Click),
        );
        examples.insert(
            "mousemove",
            enigo::agent::Token::MoveMouse(300, -500, enigo::Coordinate::Abs),
        );
        println!("examples as toml: {}", toml::to_string(&examples)?);

        let text_token: Token = toml::from_str(
            r#"
           Text="Hello"
        "#,
        )
        .unwrap();
        println!("text_token: {text_token:?}");
        let key_token: Token = toml::from_str(
            r#"
           Key = ["Backspace", "Press"]
        "#,
        )
        .unwrap();
        println!("key_token: {key_token:?}");
        let move_token: Token = toml::from_str(
            r#"
           MoveMouse = [300, -500, "Abs"]
        "#,
        )
        .unwrap();
        println!("move_token: {move_token:?}");

        let button_token: Token = toml::from_str(
            r#"
            Button = ["Middle", "Click"]
        "#,
        )
        .unwrap();
        println!("button_token: {button_token:?}");

        let presets = parse_toml_file(
            r#"
[PresetAtRoot]
description="Our awesome preset"
actions = [
    {Text="Hello"},
    {Key = ["Backspace", "Press"]},
    {MoveMouse = [300, -500, "Abs"]}
]
[Menu.Difficulty.Normal]
description="Our awesome preset"
actions = [
    {Text="Hello"},
    {Key = ["Backspace", "Press"]},
    {MoveMouse = [300, -500, "Abs"]}
]
[Menu.Difficulty.Nightmare]
description="Our awesome preset"
actions = [
    {Text="Hello"},
    {Key = ["Backspace", "Press"]},
    {MoveMouse = [300, -500, "Abs"]}
]
        "#,
        )?;

        println!("Presets: {presets:#?}");

        Ok(())
    }
}
