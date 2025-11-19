use anstyle::{AnsiColor, Color, RgbColor, Style};
use clap::builder::Styles;

use crate::utils::{CHECK, MARK};

// Brand colors
#[allow(dead_code)]
const PRIMARY: RgbColor = RgbColor(0, 43, 86); // #002B56
const SECONDARY: RgbColor = RgbColor(24, 66, 137); // #184289
const TERTIARY: RgbColor = RgbColor(30, 105, 105); // #1E6969
const TEXT_COLOR: RgbColor = RgbColor(1, 46, 88); // #012E58

pub(crate) fn get_styles() -> Styles {
    Styles::styled()
        .usage(
            Style::new()
                .bold()
                .underline()
                .fg_color(Some(Color::Rgb(SECONDARY))),
        )
        .header(
            Style::new()
                .bold()
                .underline()
                .fg_color(Some(Color::Rgb(SECONDARY))),
        )
        .literal(Style::new().fg_color(Some(Color::Rgb(TERTIARY))))
        .invalid(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::Red))),
        )
        .error(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::Red))),
        )
        .valid(
            Style::new()
                .bold()
                .underline()
                .fg_color(Some(Color::Rgb(TERTIARY))),
        )
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::White))))
}

fn fmt_style(msg: &str, style: &Style) -> String {
    format!("{style}{msg}{style:#}")
}

#[allow(dead_code)]
pub(crate) fn fmt_primary(msg: &str) -> String {
    let style = Style::new().fg_color(Some(Color::Rgb(PRIMARY)));
    fmt_style(msg, &style)
}

pub(crate) fn fmt_secondary(msg: &str) -> String {
    let style = Style::new().fg_color(Some(Color::Rgb(SECONDARY)));
    fmt_style(msg, &style)
}

pub(crate) fn fmt_tertiary(msg: &str) -> String {
    let style = Style::new().fg_color(Some(Color::Rgb(TERTIARY)));
    fmt_style(msg, &style)
}

#[allow(dead_code)]
pub(crate) fn fmt_text(msg: &str) -> String {
    let style = Style::new().fg_color(Some(Color::Rgb(TEXT_COLOR)));
    fmt_style(msg, &style)
}

// Legacy color functions - map to brand colors
pub(crate) fn fmt_green(msg: &str) -> String {
    fmt_tertiary(msg)
}

pub(crate) fn fmt_cyan(msg: &str) -> String {
    fmt_secondary(msg)
}

pub(crate) fn fmt_red(msg: &str) -> String {
    let red = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));
    fmt_style(msg, &red)
}

pub(crate) fn fmt_yellow(msg: &str) -> String {
    let yellow = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
    fmt_style(msg, &yellow)
}

pub(crate) fn fmt_bold(msg: &str) -> String {
    let bold = Style::new().bold().fg_color(Some(Color::Rgb(TEXT_COLOR)));
    fmt_style(msg, &bold)
}

pub(crate) fn fmt_dimmed(msg: &str) -> String {
    let dimmed = Style::new().dimmed();
    fmt_style(msg, &dimmed)
}

pub(crate) fn fmt_success(msg: &str) -> String {
    format!("{} {msg}", fmt_tertiary(CHECK))
}

pub(crate) fn fmt_error(msg: &str) -> String {
    format!("{} {msg}", fmt_red(MARK))
}
