//! THE CONTENT OF A WINDOW KEEPS CLEAR OF THE LINE THAT DIVIDES IT FROM ITS SIDE PANEL.
//!
//! Reported behaviour (issue #16): in the settings window the text of a section was pressed against the line
//! between the list of sections and the section itself - "General", "Language", the highlight of "English"
//! all started on the line, with not a pixel between.
//!
//! The cause is how `egui::Panel` behaves INSIDE a window: the panel keeps a margin of its own, then moves the
//! cursor of the window exactly to its outer edge and draws the divider there. Whatever is added next starts
//! on that line unless it brings a margin of its own - a `CentralPanel`, as the parts library does. The
//! settings and the help drew their right side bare.
//!
//! Checked BY GEOMETRY, where the letters and the highlights actually landed, over every window that has a
//! side panel: a margin set up in one place and bypassed by the first `ui.label` outside it would pass a guard
//! over the source.
#[cfg(test)]
mod tests {
    use crate::gui::{App, WinKind};

    /// How much room the content must leave between itself and the divider. The side panel keeps 8 points on
    /// its side of the line; a little less on the other side is still a gap, nothing at all is not.
    const CLEAR: f32 = 6.0;

    /// The leftmost captions and filled backgrounds painted to the right of the side panel `panel`, below its
    /// top edge, as distances from the divider - the panel's outer right edge.
    fn gaps(app: &mut App, panel: &str, draw: impl Fn(&mut App, &egui::Context)) -> Vec<(f32, String)> {
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1400.0, 900.0));
        let input = egui::RawInput { screen_rect: Some(screen), ..Default::default() };
        let ctx = egui::Context::default();
        crate::gui::install_fonts(&ctx);
        let _ = ctx.run_ui(input.clone(), |c| draw(app, c.ctx()));
        let out = ctx.run_ui(input, |c| draw(app, c.ctx()));
        let side = egui::PanelState::load(&ctx, egui::Id::new(panel)).unwrap_or_else(|| panic!("the panel \"{panel}\" was not drawn")).outer_rect;

        // A filled background counts when it is the size of one row - a highlight, a button. The ground of a
        // whole area is painted from the divider on by design: that is the panel, not its content.
        fn collect(s: &egui::Shape, out: &mut Vec<(egui::Rect, String)>) {
            match s {
                egui::Shape::Text(t) => out.push((egui::Rect::from_min_size(t.pos, t.galley.size()), t.galley.text().to_string())),
                egui::Shape::Rect(r) if r.fill.a() > 0 && r.rect.height() <= 40.0 => out.push((r.rect, "a filled background".to_string())),
                egui::Shape::Vec(v) => v.iter().for_each(|x| collect(x, out)),
                _ => {}
            }
        }
        let mut found = Vec::new();
        for cs in &out.shapes {
            collect(&cs.shape, &mut found);
        }
        let divider = side.max.x;
        // A shape that starts a point or two left of the line still belongs to the right side: a highlight is
        // expanded around its widget.
        found
            .into_iter()
            .filter(|(r, text)| r.min.x > divider - 2.0 && r.min.y >= side.min.y && r.max.y <= side.max.y && !text.trim().is_empty())
            .map(|(r, text)| (r.min.x - divider, text))
            .collect()
    }

    fn assert_clear(name: &str, gaps: Vec<(f32, String)>) {
        assert!(gaps.len() > 3, "{name}: suspiciously little is painted to the right of the side panel: {}", gaps.len());
        let (gap, what) = gaps.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)).expect("checked above");
        assert!(gap >= CLEAR, "{name}: {what:?} is {gap:.1} points from the divider of the side panel, and there must be at least {CLEAR}");
    }

    #[test]
    fn the_settings_section_keeps_clear_of_the_list_of_sections() {
        let mut app = App::default();
        app.win.open(WinKind::Settings);
        let g = gaps(&mut app, "settings_sections", |a, c| {
            let mut asks = Vec::new();
            crate::gui::panels_windows::settings_window(&mut a.win_ctx(&mut asks), c);
            a.do_win_asks(asks, c);
        });
        assert_clear("the settings", g);
    }

    #[test]
    fn the_help_article_keeps_clear_of_the_contents() {
        let mut app = App::default();
        app.open_help("assembly/02-joints");
        let g = gaps(&mut app, "help_toc", |a, c| a.help_window(c));
        assert_clear("the help", g);
    }

    #[test]
    fn the_parts_keep_clear_of_the_catalogue() {
        let mut app = App::default();
        app.win.open(WinKind::PartsLibrary);
        // a search lists the matching parts on the right; with no category chosen there is one hint and nothing more
        app.parts.search = "e".into();
        let g = gaps(&mut app, "parts_lib_tree", |a, c| {
            let mut asks = Vec::new();
            crate::gui::panels_windows::parts_library_window(&mut a.win_ctx(&mut asks), c);
            a.do_win_asks(asks, c);
        });
        assert_clear("the parts library", g);
    }
}
