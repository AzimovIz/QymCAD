//! A PATTERN ACROSS A PATTERN IS LAID BY HAND, through the window: a line and another across it, each patterned by the
//! linear pattern tool - the count and the steps typed in its bar, Enter - make a grid of cells, and the frames over it
//! stay quick.
//!
//! Reported behaviour (#95): one line horizontal and one vertical, each patterned 200 times 20 mm apart, and every
//! operation of the sketch waited 30 s or more. Each copy of a pattern rebuilt the loops of the whole growing grid.
#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::{App, Sel};
    use qymcad_core::feature::SketchPlane;
    use std::time::{Duration, Instant};

    #[test]
    fn a_grid_of_two_patterns_is_laid_and_worked() {
        // A RELEASE BUILD TAKES THE GRID OF THE REPORT, 200 by 200; a test build 40 by 40
        let release = !cfg!(debug_assertions);
        let n: u32 = if release { 200 } else { 40 };
        let len = 20.0 * n as f64;
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        let mut hand = Hand::new(&mut app);
        hand.sk_tool(1).click2d(0.0, -10.0).click2d(len, -10.0).key(egui::Key::Escape).key(egui::Key::Escape);
        hand.sk_tool(1).click2d(-10.0, 0.0).click2d(-10.0, len).key(egui::Key::Escape).key(egui::Key::Escape);
        let mut laid = Vec::new();
        for (middle, step) in [((len / 2.0, -10.0), ["0", "20"]), ((-10.0, len / 2.0), ["20", "0"])] {
            hand.sk_tool(0);
            hand.click2d(middle.0, middle.1);
            assert!(hand.press_hint(&crate::i18n::tr("tb-lin-array-hint")), "the linear pattern");
            assert!(hand.type_after_word(&crate::i18n::tr("opt-count"), &n.to_string()), "the count of the pattern");
            assert!(hand.type_after_word(&crate::i18n::tr("opt-step-x"), step[0]), "the step X");
            assert!(hand.type_after_word(&crate::i18n::tr("opt-step-y"), step[1]), "the step Y");
            let started = Instant::now();
            hand.key(egui::Key::Enter);
            laid.push(started.elapsed());
            hand.key(egui::Key::Escape);
        }
        let s = &hand.app.project.sketches[si];
        assert_eq!(s.entities.len(), 2 * n as usize, "the two patterns laid their copies");
        let cells = ((n - 1) * (n - 1)) as usize;
        assert!(s.contour_ids.len() >= cells, "the grid has {} loops, fewer than its {cells} cells", s.contour_ids.len());
        let started = Instant::now();
        hand.hover2d(15.0, 15.0);
        let hover = started.elapsed();
        eprintln!("a grid of {n} by {n}: the patterns laid in {laid:?}, two frames with the pointer over it {hover:?}");
        // measured in a release build on 200 by 200: the second pattern 29 s rebuilt after each copy, 0.3 s rebuilt once
        assert!(laid.iter().all(|t| *t < Duration::from_secs(2)), "a pattern of the grid took {laid:?}, budget 2 s");
    }
}
