//! A RECTANGLE DRAGGED BY HAND CHANGES ITS SIZE FROM WHERE IT WAS DRAWN, in real frames of the window: drawn by its
//! corners, the corner it was drawn from stays when another corner is dragged, and the corner across from it stays
//! when that corner itself is dragged; drawn from its centre, the centre stays under a dragged corner, and a dragged
//! centre carries the whole rectangle.
//!
//! Reported behaviour (issue #56): on a rectangle drawn from a corner, dragging a corner next to that corner moved it
//! too, and dragging the corner it was drawn from moved every other corner.
#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::{App, Sel};
    use qymcad_core::feature::SketchPlane;

    /// A rectangle (0, 0) - (40, 30) drawn by `mode` - its two clicks, the size window closed with Enter - and the select
    /// tool taken.
    fn a_rectangle(mode: &str, clicks: [(f64, f64); 2]) -> (App, usize) {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        let mut hand = Hand::new(&mut app);
        hand.sk_tool(2);
        assert!(hand.press_word(&qymcad_i18n::tr(mode), egui::pos2(0.0, 0.0)), "no {mode} on the bar");
        hand.click2d(clicks[0].0, clicks[0].1).click2d(clicks[1].0, clicks[1].1).key(egui::Key::Enter);
        Hand::new(&mut app).sk_tool(0);
        assert_eq!(app.project.sketches[si].rects.len(), 1, "the rectangle was not drawn as one shape");
        (app, si)
    }

    /// Where the point of the sketch nearest to `at` stands now, `None` when there is none within 1 mm.
    fn point_near(app: &App, si: usize, id: u64) -> (f64, f64) {
        let q = app.project.sketches[si].points.iter().find(|q| q.id == id).expect("the point is there");
        (q.x, q.y)
    }

    fn id_at(app: &App, si: usize, at: (f64, f64)) -> u64 {
        app.project.sketches[si].points.iter().filter(|q| (q.x - at.0).hypot(q.y - at.1) < 1e-6).map(|q| q.id).next().expect("a corner there")
    }

    fn stayed(app: &App, si: usize, id: u64, at: (f64, f64)) -> Option<String> {
        let now = point_near(app, si, id);
        ((now.0 - at.0).hypot(now.1 - at.1) > 1e-3).then(|| format!("went from {at:?} to {now:?}"))
    }

    #[test]
    fn a_dragged_rectangle_holds_where_it_was_drawn_from() {
        let mut sins = Vec::new();
        // drawn by its corners from (0, 0): a corner next to it, and the corner across, dragged - (0, 0) stays
        for (from, to) in [((40.0, 0.0), (48.0, -6.0)), ((0.0, 30.0), (-6.0, 38.0)), ((40.0, 30.0), (48.0, 38.0))] {
            let (mut app, si) = a_rectangle("opt-rect-2corners", [(0.0, 0.0), (40.0, 30.0)]);
            let anchor = id_at(&app, si, (0.0, 0.0));
            Hand::new(&mut app).drag2d(from, to);
            if let Some(why) = stayed(&app, si, anchor, (0.0, 0.0)) {
                sins.push(format!("drawn from (0, 0), the corner {from:?} dragged: the corner drawn from {why}"));
            }
        }
        // drawn from the top left corner (0, 30), as reported: its neighbours and the corner across dragged - (0, 30) stays
        for (from, to) in [((40.0, 30.0), (48.0, 38.0)), ((0.0, 0.0), (-6.0, -8.0)), ((40.0, 0.0), (48.0, -6.0))] {
            let (mut app, si) = a_rectangle("opt-rect-2corners", [(0.0, 30.0), (40.0, 0.0)]);
            let anchor = id_at(&app, si, (0.0, 30.0));
            Hand::new(&mut app).drag2d(from, to);
            if let Some(why) = stayed(&app, si, anchor, (0.0, 30.0)) {
                sins.push(format!("drawn from (0, 30), the corner {from:?} dragged: the corner drawn from {why}"));
            }
        }
        // the corner drawn from dragged itself: the corner across from it stays
        let (mut app, si) = a_rectangle("opt-rect-2corners", [(0.0, 0.0), (40.0, 30.0)]);
        let across = id_at(&app, si, (40.0, 30.0));
        Hand::new(&mut app).drag2d((0.0, 0.0), (-6.0, -8.0));
        if let Some(why) = stayed(&app, si, across, (40.0, 30.0)) {
            sins.push(format!("drawn from (0, 0), that corner dragged: the corner across {why}"));
        }
        // drawn from its centre (20, 15): a corner dragged - the centre stays
        let (mut app, si) = a_rectangle("opt-rect-centre", [(20.0, 15.0), (40.0, 30.0)]);
        let centre = app.project.sketches[si].rects[0].centre;
        Hand::new(&mut app).drag2d((40.0, 30.0), (46.0, 36.0));
        if let Some(why) = stayed(&app, si, centre, (20.0, 15.0)) {
            sins.push(format!("drawn from the centre, a corner dragged: the centre {why}"));
        }
        assert!(sins.is_empty(), "{}", sins.join("\n"));
    }
}
