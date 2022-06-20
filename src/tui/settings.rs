use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColorType {
    Error,
    CommandDelimiters,
    Dollar,
    Property,
    String,
    Number,
    CommandName,
    AbsentCommandName,
}

pub struct ColorScheme {
    data: HashMap<ColorType, String>,
}

impl ColorScheme {
    pub fn new() -> Self {
        let mut data: HashMap<ColorType, String> = HashMap::new();

        use termion::color::*;
        data.insert(ColorType::Error, Fg(Red).to_string());
        data.insert(ColorType::CommandDelimiters, Fg(Cyan).to_string());
        data.insert(ColorType::Dollar, Fg(Yellow).to_string());
        data.insert(ColorType::Property, Fg(LightYellow).to_string());
        data.insert(ColorType::String, Fg(LightGreen).to_string());
        data.insert(ColorType::Number, Fg(LightGreen).to_string());
        data.insert(ColorType::CommandName, Fg(LightGreen).to_string());
        data.insert(ColorType::AbsentCommandName, Fg(LightGreen).to_string());

        return Self {
            data
        };
    }

    pub fn get(&self, color_type: &ColorType) -> &str {
        return self.data.get(color_type).map(|a| a.as_str()).unwrap_or("");
    }

}

pub struct TUISettings {
    color_scheme: ColorScheme,
}

impl TUISettings {
    pub fn new() -> Self {
        return Self {
            color_scheme: ColorScheme::new(),
        };
    }

    pub fn color_scheme(&self) -> &ColorScheme {
        &self.color_scheme
    }
}

