use jiff::Timestamp;
use jiff::civil::date;

use super::*;

fn entry(start_min: i64, minutes: i64) -> ExportRow {
    ExportRow::Entry {
        start: Timestamp::from_second(start_min * 60).unwrap(),
        end: Timestamp::from_second((start_min + minutes) * 60).unwrap(),
        category: Some("Work".into()),
        activity: "Code review".into(),
        source: "prompt".into(),
    }
}

fn correction(minutes: i64) -> ExportRow {
    ExportRow::Correction {
        day: date(2026, 10, 2),
        category: None,
        activity: "Planning".into(),
        seconds: minutes * 60,
    }
}

fn rows() -> Vec<ExportRow> {
    let mut rows: Vec<ExportRow> = (0..6).map(|i| entry(i * 60, 45)).collect();
    rows.push(correction(30));
    rows
}

#[test]
fn csv_preview_shows_the_header_and_counts() {
    let preview = preview(&rows(), Format::Csv);
    assert_eq!(preview.lines.len(), 5);
    assert_eq!(
        preview.lines[0],
        "kind,start,end,day,category,activity,minutes"
    );
    assert_eq!(preview.more, 8 - 5);
    assert_eq!(preview.entries, 6);
    assert_eq!(preview.corrections, 1);
    assert_eq!(preview.seconds, (6 * 45 + 30) * 60);
}

#[test]
fn json_preview_is_the_pretty_file() {
    let preview = preview(&rows(), Format::Json);
    assert_eq!(preview.lines[0], "[");
    assert!(preview.more > 0);
}

#[test]
fn rendering_matches_the_core_exporters() {
    let csv = render(&rows(), Format::Csv).unwrap();
    assert!(
        String::from_utf8(csv)
            .unwrap()
            .ends_with("correction,,,2026-10-02,,Planning,30.00\n")
    );
    let json = render(&rows(), Format::Json).unwrap();
    assert!(
        String::from_utf8(json)
            .unwrap()
            .contains("\"kind\": \"correction\"")
    );
}

#[test]
fn file_names_describe_the_range() {
    let day = date(2026, 10, 7);
    let name = |unit, format| file_name(RangeSpec::new(unit, day), format);
    assert_eq!(name(RangeUnit::Day, Format::Csv), "tally-2026-10-07.csv");
    assert_eq!(
        name(RangeUnit::Week, Format::Json),
        "tally-week-2026-10-05.json"
    );
    assert_eq!(name(RangeUnit::Month, Format::Csv), "tally-2026-10.csv");
    assert_eq!(name(RangeUnit::Year, Format::Csv), "tally-2026.csv");
    assert_eq!(name(RangeUnit::All, Format::Json), "tally-all.json");
}

#[test]
fn export_needs_rows() {
    let mut export = Export::new(date(2026, 10, 7));
    assert_eq!(export.update(Message::Export), None);
    export.set_rows(rows(), date(2026, 10, 7));
    assert_eq!(
        export.update(Message::Export),
        Some(Outcome::Save {
            file_name: "tally-2026-10.csv".into()
        })
    );
}

#[test]
fn switching_format_updates_the_preview() {
    let mut export = Export::new(date(2026, 10, 7));
    export.set_rows(rows(), date(2026, 10, 7));
    export.update(Message::Format(Format::Json));
    assert_eq!(export.preview.lines[0], "[");
    assert_eq!(export.format(), Format::Json);
}

#[test]
fn cannot_export_the_future() {
    let mut export = Export::new(date(2026, 10, 7));
    assert_eq!(export.update(Message::Step(1)), None);
    assert_eq!(
        export.update(Message::Step(-1)),
        Some(Outcome::RangeChanged)
    );
    assert_eq!(export.spec().anchor, date(2026, 9, 1));
}
