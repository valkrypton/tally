//! The line icons from the artboards, as inline SVG drawn by iced's `svg` widget.
//! `currentColor` is replaced at draw time by `theme::icon_color`.

use iced::widget::svg;

macro_rules! icon {
    ($name:ident, $width:literal, $path:literal) => {
        pub fn $name() -> svg::Handle {
            svg::Handle::from_memory(
                concat!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width=""#,
                    $width,
                    r#"" stroke-linecap="round" stroke-linejoin="round">"#,
                    $path,
                    "</svg>"
                )
                .as_bytes(),
            )
        }
    };
}

icon!(chevron_left, "2", r#"<path d="M15 18l-6-6 6-6"/>"#);
icon!(chevron_right, "2", r#"<path d="M9 18l6-6-6-6"/>"#);
icon!(chevron_down, "2", r#"<path d="M6 9l6 6 6-6"/>"#);
icon!(check, "2.5", r#"<path d="M5 12l5 5 9-10"/>"#);
icon!(plus, "2.5", r#"<path d="M12 5v14M5 12h14"/>"#);
icon!(minus, "2.5", r#"<path d="M5 12h14"/>"#);
icon!(pause, "2", r#"<path d="M9 5v14M15 5v14"/>"#);
icon!(play, "2", r#"<path d="M8 5l11 7-11 7z"/>"#);
icon!(menu, "2", r#"<path d="M4 7h16M4 12h16M4 17h16"/>"#);
icon!(
    alert,
    "2",
    r#"<path d="M12 8v5M12 16.5v.5"/><path d="M10.3 3.9L2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z"/>"#
);
