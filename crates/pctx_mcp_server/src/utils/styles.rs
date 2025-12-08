use anstyle::{Color, RgbColor, Style};

// Brand colors
const SECONDARY: RgbColor = RgbColor(24, 66, 137); // #184289

pub(crate) fn fmt_cyan(msg: &str) -> String {
    let style = Style::new().fg_color(Some(Color::Rgb(SECONDARY)));
    format!("{style}{msg}{style:#}")
}

pub(crate) fn fmt_dimmed(msg: &str) -> String {
    let style = Style::new().dimmed();
    format!("{style}{msg}{style:#}")
}
