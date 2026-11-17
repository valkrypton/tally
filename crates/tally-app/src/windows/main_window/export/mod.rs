//! Export: CSV or JSON for a chosen range, saved wherever the user picks.
//! Artboard B-Export.

use iced::alignment::Vertical;
use iced::widget::{Space, column, container, radio, row, text};
use iced::{Element, Length, Padding};
use jiff::civil::Date;
use tally_core::export::{ExportRow, to_json, write_csv};
use tally_core::range::{RangeSpec, RangeUnit};

use crate::windows::dashboard::step_within;
use crate::windows::widgets::{self, primary_button, step_arrow};
use crate::{format, icons, theme};

/// Lines of the file shown in the preview.
const PREVIEW_LINES: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Csv,
    Json,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Csv => "csv",
            Format::Json => "json",
        }
    }
}

/// The file's bytes.
pub fn render(rows: &[ExportRow], format: Format) -> tally_core::Result<Vec<u8>> {
    match format {
        Format::Csv => {
            let mut out = Vec::new();
            write_csv(rows, &mut out)?;
            Ok(out)
        }
        Format::Json => Ok(to_json(rows)?.into_bytes()),
    }
}

/// What the preview block shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Preview {
    pub lines: Vec<String>,
    /// Lines not shown.
    pub more: usize,
    pub entries: usize,
    pub corrections: usize,
    pub seconds: i64,
}

pub fn preview(rows: &[ExportRow], format: Format) -> Preview {
    let rendered = render(rows, format).unwrap_or_default();
    let text = String::from_utf8_lossy(&rendered);
    let all: Vec<&str> = text.lines().collect();
    Preview {
        lines: all
            .iter()
            .take(PREVIEW_LINES)
            .map(|l| (*l).to_owned())
            .collect(),
        more: all.len().saturating_sub(PREVIEW_LINES),
        entries: rows
            .iter()
            .filter(|r| matches!(r, ExportRow::Entry { .. }))
            .count(),
        corrections: rows
            .iter()
            .filter(|r| matches!(r, ExportRow::Correction { .. }))
            .count(),
        seconds: rows.iter().map(ExportRow::seconds).sum(),
    }
}

/// A file name for the save dialog: `tally-2026-10.csv`, `tally-2026-10-05.json`, ...
pub fn file_name(spec: RangeSpec, format: Format) -> String {
    let stem = match spec.days() {
        Ok((first, _)) => match spec.unit {
            RangeUnit::Day => first.strftime("%Y-%m-%d").to_string(),
            RangeUnit::Week => format!("week-{}", first.strftime("%Y-%m-%d")),
            RangeUnit::Month => first.strftime("%Y-%m").to_string(),
            RangeUnit::Year => first.year().to_string(),
            RangeUnit::All => String::from("all"),
        },
        Err(_) => String::from("all"),
    };
    format!("tally-{stem}.{}", format.extension())
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{} {}", count, if count == 1 { one } else { many })
}

#[derive(Debug, Clone)]
pub enum Message {
    Unit(RangeUnit),
    Step(i32),
    Format(Format),
    Export,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Reload rows for the new range.
    RangeChanged,
    /// Ask where to save, then write the file.
    Save { file_name: String },
}

#[derive(Debug, Clone)]
pub struct Export {
    spec: RangeSpec,
    today: Date,
    format: Format,
    rows: Vec<ExportRow>,
    preview: Preview,
    /// The last save: the path written, or why it failed.
    result: Option<Result<String, String>>,
}

impl Export {
    pub fn new(today: Date) -> Self {
        Self {
            spec: RangeSpec::new(RangeUnit::Month, today),
            today,
            format: Format::Csv,
            rows: Vec::new(),
            preview: Preview::default(),
            result: None,
        }
    }

    pub fn spec(&self) -> RangeSpec {
        self.spec
    }

    pub fn format(&self) -> Format {
        self.format
    }

    pub fn rows(&self) -> &[ExportRow] {
        &self.rows
    }

    pub fn set_rows(&mut self, rows: Vec<ExportRow>, today: Date) {
        self.today = today;
        self.preview = preview(&rows, self.format);
        self.rows = rows;
    }

    pub fn saved(&mut self, result: Result<String, String>) {
        self.result = Some(result);
    }

    pub fn update(&mut self, message: Message) -> Option<Outcome> {
        self.result = None;
        match message {
            Message::Unit(unit) => {
                self.spec = RangeSpec::new(unit, self.spec.anchor);
                Some(Outcome::RangeChanged)
            }
            Message::Step(steps) => {
                self.spec = step_within(self.spec, steps, self.today)?;
                Some(Outcome::RangeChanged)
            }
            Message::Format(format) => {
                self.format = format;
                self.preview = preview(&self.rows, format);
                None
            }
            Message::Export => (!self.rows.is_empty()).then(|| Outcome::Save {
                file_name: file_name(self.spec, self.format),
            }),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let (title, subtitle) = format::range_title(self.spec, self.today);
        let label = |s| theme::small(s).color(theme::TEXT_MUTED).width(96);
        let step = |handle, steps| step_arrow(handle, self.spec, steps, self.today, Message::Step);
        let range = row![
            label("Range"),
            widgets::unit_picker(self.spec.unit, false, Message::Unit),
            step(icons::chevron_left(), -1),
            column![
                text(title).font(theme::UI_SEMIBOLD).size(14),
                theme::label(subtitle),
            ],
            step(icons::chevron_right(), 1),
        ]
        .spacing(theme::S_MD)
        .align_y(Vertical::Center);
        let option = |format: Format, name: &'static str, note: &'static str| {
            row![
                radio(name, format, Some(self.format), Message::Format)
                    .size(16)
                    .text_size(14)
                    .font(theme::UI)
                    .style(theme::radio_style),
                theme::label(note),
            ]
            .spacing(theme::S_SM)
            .align_y(Vertical::Center)
        };
        let formats = row![
            label("Format"),
            option(Format::Csv, "CSV", "for spreadsheets"),
            option(Format::Json, "JSON", "for scripts"),
        ]
        .spacing(theme::S_XL)
        .align_y(Vertical::Center);

        let p = &self.preview;
        let counts = row![
            text(p.entries.to_string()).font(theme::MONO).size(12),
            theme::label(format!(
                " {}, ",
                if p.entries == 1 { "entry" } else { "entries" }
            )),
            text(p.corrections.to_string()).font(theme::MONO).size(12),
            theme::label(format!(
                " {}, ",
                if p.corrections == 1 {
                    "correction"
                } else {
                    "corrections"
                }
            )),
            text(format::duration(p.seconds)).font(theme::MONO).size(12),
            theme::label(" in total"),
        ];
        let mut block = column![].spacing(theme::S_XS);
        if self.rows.is_empty() {
            block = block.push(theme::label("Nothing to export in this range."));
        } else {
            for (index, line) in p.lines.iter().enumerate() {
                let color = if index == 0 && self.format == Format::Csv {
                    theme::TEXT_MUTED
                } else {
                    theme::TEXT_PRIMARY
                };
                block = block.push(text(line.clone()).font(theme::MONO).size(12).color(color));
            }
            if p.more > 0 {
                block = block.push(
                    text(format!("… {}", plural(p.more, "more line", "more lines")))
                        .font(theme::MONO)
                        .size(12)
                        .color(theme::TEXT_MUTED),
                );
            }
        }
        let preview = column![
            row![
                text("Preview")
                    .font(theme::UI_SEMIBOLD)
                    .size(13)
                    .width(Length::Fill),
                counts
            ]
            .align_y(Vertical::Center),
            container(block)
                .padding(Padding::from([12.0, 14.0]))
                .width(Length::Fill)
                .style(theme::preview_block),
            theme::label(
                "Times are in UTC. Corrections are time you added by hand, so they have a \
                 day but no start or end."
            ),
        ]
        .spacing(theme::S_SM);

        let note = match &self.result {
            Some(Ok(path)) => theme::label(format!("Saved to {path}")),
            Some(Err(error)) => theme::label(format!("Couldn't save: {error}")).color(theme::ALERT),
            None => theme::label("You'll choose where to save it."),
        };
        let action = row![
            Space::new().width(Length::Fill),
            note,
            primary_button(
                format!("Export {}…", self.format.extension().to_uppercase()),
                None,
                None,
                (!self.rows.is_empty()).then_some(Message::Export),
            ),
        ]
        .spacing(theme::S_MD)
        .align_y(Vertical::Center);

        column![
            theme::headline("Export"),
            column![range, formats].spacing(theme::S_LG),
            preview,
            Space::new().height(Length::Fill),
            action,
        ]
        .spacing(22)
        .height(Length::Fill)
        .into()
    }
}

#[cfg(test)]
mod tests;
