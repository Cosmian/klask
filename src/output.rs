use cansi::{CategorisedSlice, Color};
use eframe::egui::{vec2, Color32, Label, ProgressBar, Ui};
use linkify::{LinkFinder, LinkKind};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Displays a progress bar in the output. First call creates
/// a progress bar and future calls update it.
///
/// Value is a f32 between 0 and 1.
///
/// If the description is not static, use [`progress_bar_with_id`].
/// ```no_run
/// # use clap::{App, Arg};
/// # use klask::Settings;
/// fn main() {
///     klask::run_app(App::new("Example"), Settings::default(), |matches| {
///         for i in 0..=100 {
///             klask::output::progress_bar("Static description", i as f32 / 100.0);
///         }
///     });
/// }
/// ```
pub fn progress_bar(description: &str, value: f32) {
    progress_bar_with_id(description, description, value)
}

/// Displays a progress bar in the output. First call creates
/// a progress bar and future calls update it.
///
/// Value is a f32 between 0 and 1.
/// Id is any hashable value that uniquely identifies a progress bar.
/// ```no_run
/// # use clap::{App, Arg};
/// # use klask::Settings;
/// fn main() {
///     klask::run_app(App::new("Example"), Settings::default(), |matches| {
///         for i in 0..=100 {
///             klask::output::progress_bar_with_id(
///                 "Progress",
///                 &format!("Dynamic description [{}/{}]", i, 100),
///                 i as f32 / 100.0,
///             );
///         }
///     });
/// }
/// ```
pub fn progress_bar_with_id(id: impl Hash, description: &str, value: f32) {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    OutputType::ProgressBar(description.to_string(), value).send(h.finish());
}

#[derive(Debug)]
pub(crate) struct Output(Vec<(u64, OutputType)>);

#[derive(Debug)]
pub(crate) enum OutputType {
    Text(String),
    ProgressBar(String, f32),
}

impl Output {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn parse(&mut self, str: &str) {
        let mut iter = str.split(MAGIC);

        if let Some(text) = iter.next() {
            if !text.is_empty() {
                self.0.push((0, OutputType::Text(text.to_string())))
            }
        }

        while let Some(id) = iter.next() {
            if let Ok(id) = id.parse() {
                if let Some(new) = OutputType::parse(&mut iter) {
                    if let Some((_, exists)) = self.0.iter_mut().find(|(i, _)| *i == id) {
                        *exists = new;
                    } else {
                        self.0.push((id, new));
                    }
                }
            }

            if let Some(text) = iter.next() {
                // Get rid of the newline
                let text = &text[1..];
                if !text.is_empty() {
                    self.0.push((0, OutputType::Text(text.to_string())))
                }
            }
        }
    }

    pub fn update(&mut self, ui: &mut Ui) {
        for (_, o) in &mut self.0 {
            match o {
                OutputType::Text(ref text) => format_output(ui, text),
                OutputType::ProgressBar(ref mess, value) => {
                    ui.add(ProgressBar::new(*value).text(mess).animate(true));
                }
            }
        }
    }
}

/// Unicode non-character. Used for sending messages between GUI and user's program
const MAGIC: char = '\u{5FFFE}';
fn send_message(data: &[&str]) {
    for d in data {
        print!("{}{}", MAGIC, d);
    }
    println!("{}", MAGIC);
}

impl OutputType {
    const PROGRESS_BAR_STR: &'static str = "progress-bar";

    pub fn send(self, id: u64) {
        match self {
            OutputType::Text(s) => print!("{}", s),
            OutputType::ProgressBar(desc, value) => send_message(&[
                &id.to_string(),
                Self::PROGRESS_BAR_STR,
                &desc,
                &value.to_string(),
            ]),
        }
    }

    pub fn parse<'a>(iter: &mut impl Iterator<Item = &'a str>) -> Option<Self> {
        match iter.next() {
            Some(Self::PROGRESS_BAR_STR) => Some(Self::ProgressBar(
                iter.next().unwrap_or_default().to_string(),
                iter.next()
                    .map(|s| s.parse().ok())
                    .flatten()
                    .unwrap_or_default(),
            )),
            None => None,
            _ => panic!(),
        }
    }
}

fn format_output(ui: &mut Ui, text: &str) {
    let output = cansi::categorise_text(text);

    let previous = ui.style().spacing.item_spacing;
    ui.style_mut().spacing.item_spacing = vec2(0.0, 0.0);

    ui.horizontal_wrapped(|ui| {
        for CategorisedSlice {
            text,
            fg_colour,
            bg_colour,
            intensity,
            italic,
            underline,
            strikethrough,
            ..
        } in output
        {
            for span in LinkFinder::new().spans(text) {
                match span.kind() {
                    Some(LinkKind::Url) => ui.hyperlink(span.as_str()),
                    Some(LinkKind::Email) => {
                        ui.hyperlink_to(span.as_str(), format!("mailto:{}", span.as_str()))
                    }
                    Some(_) | None => {
                        let mut label = Label::new(span.as_str());

                        label = label.text_color(ansi_color_to_egui(fg_colour));

                        if bg_colour != Color::Black {
                            label = label.background_color(ansi_color_to_egui(bg_colour));
                        }

                        if italic {
                            label = label.italics();
                        }

                        if underline {
                            label = label.underline();
                        }

                        if strikethrough {
                            label = label.strikethrough();
                        }

                        label = match intensity {
                            cansi::Intensity::Normal => label,
                            cansi::Intensity::Bold => label.strong(),
                            cansi::Intensity::Faint => label.weak(),
                        };

                        ui.add(label)
                    }
                };
            }
        }
    });
    ui.style_mut().spacing.item_spacing = previous;
}

fn ansi_color_to_egui(color: Color) -> Color32 {
    match color {
        Color::Black => Color32::from_rgb(0, 0, 0),
        Color::Red => Color32::from_rgb(205, 49, 49),
        Color::Green => Color32::from_rgb(13, 188, 121),
        Color::Yellow => Color32::from_rgb(229, 229, 16),
        Color::Blue => Color32::from_rgb(36, 114, 200),
        Color::Magenta => Color32::from_rgb(188, 63, 188),
        Color::Cyan => Color32::from_rgb(17, 168, 205),
        Color::White => Color32::from_rgb(229, 229, 229),
        Color::BrightBlack => Color32::from_rgb(102, 102, 102),
        Color::BrightRed => Color32::from_rgb(241, 76, 76),
        Color::BrightGreen => Color32::from_rgb(35, 209, 139),
        Color::BrightYellow => Color32::from_rgb(245, 245, 67),
        Color::BrightBlue => Color32::from_rgb(59, 142, 234),
        Color::BrightMagenta => Color32::from_rgb(214, 112, 214),
        Color::BrightCyan => Color32::from_rgb(41, 184, 219),
        Color::BrightWhite => Color32::from_rgb(229, 229, 229),
    }
}
