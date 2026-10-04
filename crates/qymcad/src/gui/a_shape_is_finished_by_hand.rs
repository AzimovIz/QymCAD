//! A SHAPE IS DRAWN AND FINISHED THE WAY A PERSON FINISHES IT, IN THE WINDOW.
//!
//! A chain of lines and a spline take as many clicks as a person wants, so they end with a gesture: a double
//! click on the last place, or Esc. In a whole frame a double click is two clicks AND a double click - egui
//! reports both clicks before it reports the double one - so the drawing tool hears the last place twice.
//!
//! Held here: a chain ended by a double click carries no line of no length, a spline ended so has one node per
//! place clicked, Esc keeps the nodes of a spline as its hint says, and every drawing tool names its step of
//! undo after what it drew.
#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::{App, Sel};
    use qymcad_core::feature::SketchPlane;
    use qymcad_core::model::EntityKind;

    fn a_sketch() -> (App, usize) {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        (app, si)
    }

    /// The lines of the sketch as their ends.
    fn lines(app: &App, si: usize) -> Vec<((f64, f64), (f64, f64))> {
        let sk = &app.project.sketches[si];
        let at = |id: u64| sk.points.iter().find(|p| p.id == id).map(|p| (p.x, p.y));
        sk.entities
            .iter()
            .filter_map(|e| match e.kind {
                EntityKind::Line { a, b } => Some((at(a)?, at(b)?)),
                _ => None,
            })
            .collect()
    }

    /// The nodes of every spline of the sketch, as places.
    fn spline_nodes(app: &App, si: usize) -> Vec<Vec<(f64, f64)>> {
        let sk = &app.project.sketches[si];
        sk.splines.iter().map(|s| s.points.iter().filter_map(|id| sk.points.iter().find(|p| p.id == *id).map(|p| (p.x, p.y))).collect()).collect()
    }

    /// A DOUBLE CLICK ENDS A CHAIN OF LINES WITHOUT A LINE OF NO LENGTH.
    #[test]
    fn a_double_click_ends_a_chain_without_a_line_of_no_length() {
        let (mut app, si) = a_sketch();
        Hand::new(&mut app).sk_tool(1).click2d(0.0, 0.0).click2d(10.0, 10.0).click2d(20.0, 0.0).double_click2d(30.0, 10.0);
        let got = lines(&app, si);
        let empty: Vec<_> = got.iter().filter(|(a, b)| (a.0 - b.0).hypot(a.1 - b.1) < 1e-6).collect();
        assert!(empty.is_empty() && got.len() == 3, "the chain of three segments ended by a double click holds {} lines, of no length: {empty:?}", got.len());
        assert!(app.tools.tool.pts.is_empty(), "the chain did not end: {:?} still waits for the next click", app.tools.tool.pts);
    }

    /// A DOUBLE CLICK ENDS A SPLINE ON THE PLACE IT CLICKED - once, and as one step of undo named after it.
    #[test]
    fn a_double_click_ends_a_spline_on_the_node_it_clicked() {
        let (mut app, si) = a_sketch();
        Hand::new(&mut app).sk_tool(9).click2d(0.0, 0.0).click2d(10.0, 10.0).click2d(20.0, 0.0).double_click2d(30.0, 10.0);
        let got = spline_nodes(&app, si);
        assert_eq!(got, vec![vec![(0.0, 0.0), (10.0, 10.0), (20.0, 0.0), (30.0, 10.0)]], "four places were clicked for the spline");
        let step = app.disk.edits.undo.last().map(|s| s.name.clone());
        assert_eq!(step.as_deref(), Some(crate::i18n::tr("sk-spline").as_str()), "the spline is not one step of undo named after it");
    }

    /// THE NODES OF A SPLINE ARE DRAGGED WITH THE MOUSE in a sketch that holds nothing but the spline - the ends and a
    /// middle one - and an undo puts each back.
    ///
    /// Reported behaviour (issue #30): the tangent handle turned the curve, but a node did not move, because a sketch of
    /// splines alone was not taken for one that can be edited.
    #[test]
    fn the_nodes_of_a_lone_spline_are_dragged() {
        let drawn = vec![vec![(0.0, 0.0), (20.0, 10.0), (40.0, 0.0)]];
        let mut problems = Vec::new();
        for (node, to) in [(0usize, (0.0, -10.0)), (1, (20.0, 25.0)), (2, (45.0, -8.0))] {
            let (mut app, si) = a_sketch();
            Hand::new(&mut app).sk_tool(9).click2d(0.0, 0.0).click2d(20.0, 10.0).double_click2d(40.0, 0.0);
            assert_eq!(spline_nodes(&app, si), drawn, "setup: the spline as clicked");
            Hand::new(&mut app).sk_tool(0).drag2d(drawn[0][node], to);
            let got = spline_nodes(&app, si)[0][node];
            if (got.0 - to.0).hypot(got.1 - to.1) > 0.5 {
                problems.push(format!("node {node} dragged to {to:?} stands at {got:?}"));
            }
            Hand::new(&mut app).undo();
            if spline_nodes(&app, si) != drawn {
                problems.push(format!("node {node}: an undo leaves {:?}", spline_nodes(&app, si)));
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// A CONSTRAINT ON SEVERAL SELECTED LINES IS PUT ON EVERY ONE OF THEM: Vertical and Horizontal on each, Parallel,
    /// Equal and Collinear tying each to the first; one undo takes them all back.
    ///
    /// Reported behaviour (issue #34): with three lines selected, Vertical turned one of them and said "The constraint
    /// is added".
    #[test]
    fn a_constraint_goes_on_every_selected_line() {
        use qymcad_core::model::{Constraint, EntityKind};
        /// A constraint button, its name, and how many constraints of its kind three selected lines should get.
        struct Button {
            code: u8,
            name: &'static str,
            want: usize,
        }
        let buttons = [
            Button { code: 2, name: "Vertical", want: 3 },
            Button { code: 1, name: "Horizontal", want: 3 },
            Button { code: 3, name: "Parallel", want: 2 },
            Button { code: 5, name: "Equal", want: 2 },
            Button { code: 7, name: "Collinear", want: 2 },
        ];
        let slanted = |app: &App, si: usize| -> usize {
            let sk = &app.project.sketches[si];
            let at = |id: u64| sk.points.iter().find(|p| p.id == id).map(|p| (p.x, p.y)).expect("a point");
            sk.entities
                .iter()
                .filter(|e| match e.kind {
                    EntityKind::Line { a, b } => (at(a).0 - at(b).0).abs() > 1e-6,
                    _ => false,
                })
                .count()
        };
        let mut problems = Vec::new();
        for Button { code: button, name, want } in buttons {
            let (mut app, si) = a_sketch();
            // three slants of their own: lines drawn at one slant are tied Parallel as they are drawn, and then one
            // Vertical turns them all
            for (x, lean) in [(0.0, 5.0), (20.0, 9.0), (40.0, 3.0)] {
                Hand::new(&mut app).sk_tool(1).click2d(x, 0.0).double_click2d(x + lean, 12.0);
                Hand::new(&mut app).key(egui::Key::Escape);
            }
            let lines: Vec<(u8, u64)> = app.project.sketches[si].entities.iter().filter(|e| matches!(e.kind, EntityKind::Line { .. })).map(|e| (1u8, e.id)).collect();
            assert_eq!(lines.len(), 3, "setup: three lines");
            assert_eq!(slanted(&app, si), 3, "setup: the three lines are slanted");
            let ties = app.project.sketches[si].constraints.iter().filter(|c| matches!(c, Constraint::Parallel { .. } | Constraint::Equal { .. } | Constraint::Collinear { .. })).count();
            assert_eq!(ties, 0, "setup: nothing ties the lines together");
            let before = app.project.sketches[si].constraints.len();
            assert!(Hand::new(&mut app).select2d(&lines), "setup: the three lines are picked");
            Hand::new(&mut app).constraint(button);
            let added = app.project.sketches[si].constraints.len() - before;
            let of_kind = app.project.sketches[si].constraints[before..]
                .iter()
                .filter(|c| {
                    matches!(
                        (button, c),
                        (2, Constraint::Vertical { .. }) | (1, Constraint::Horizontal { .. }) | (3, Constraint::Parallel { .. }) | (5, Constraint::Equal { .. }) | (7, Constraint::Collinear { .. })
                    )
                })
                .count();
            if of_kind != want {
                problems.push(format!("{name} on three lines added {of_kind} of its kind ({added} in all), not {want}; status {:?}", app.status));
            }
            if button == 2 && slanted(&app, si) > 0 {
                problems.push(format!("after Vertical {} of the three lines are not vertical", slanted(&app, si)));
            }
            Hand::new(&mut app).undo();
            if app.project.sketches[si].constraints.len() != before {
                problems.push(format!("one undo after {name} leaves {} constraints, not {before}", app.project.sketches[si].constraints.len()));
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// A CONSTRAINT THAT WOULD SHRINK A LINE TO A POINT IS NOT KEPT: Horizontal on lines already Vertical is met only
    /// by a line of no length, and the sketch must say so rather than take it - one line, and three at once.
    ///
    /// Reported behaviour (found checking issue #34): three lines made Vertical, then Horizontal - the lines shrank to
    /// points and both constraints stood green.
    #[test]
    fn a_constraint_that_shrinks_a_line_to_a_point_is_refused() {
        use qymcad_core::model::{Constraint, EntityKind};
        let mut problems = Vec::new();
        for count in [1usize, 3] {
            let (mut app, si) = a_sketch();
            for (x, lean) in [(0.0, 5.0), (20.0, 9.0), (40.0, 3.0)].into_iter().take(count) {
                Hand::new(&mut app).sk_tool(1).click2d(x, 0.0).double_click2d(x + lean, 12.0);
                Hand::new(&mut app).key(egui::Key::Escape);
            }
            let lines: Vec<(u8, u64)> = app.project.sketches[si].entities.iter().filter(|e| matches!(e.kind, EntityKind::Line { .. })).map(|e| (1u8, e.id)).collect();
            assert!(Hand::new(&mut app).select2d(&lines), "setup: the lines are picked");
            Hand::new(&mut app).constraint(2);
            let vertical = app.project.sketches[si].constraints.iter().filter(|c| matches!(c, Constraint::Vertical { .. })).count();
            assert_eq!(vertical, count, "setup: every line is Vertical");
            assert!(Hand::new(&mut app).select2d(&lines), "setup: the lines are picked again");
            Hand::new(&mut app).constraint(1);
            let sk = &app.project.sketches[si];
            let at = |id: u64| sk.points.iter().find(|p| p.id == id).map(|p| (p.x, p.y)).expect("a point");
            let shortest = sk
                .entities
                .iter()
                .filter_map(|e| match e.kind {
                    EntityKind::Line { a, b } => Some((at(a).0 - at(b).0).hypot(at(a).1 - at(b).1)),
                    _ => None,
                })
                .fold(f64::MAX, f64::min);
            let horizontal = sk.constraints.iter().filter(|c| matches!(c, Constraint::Horizontal { .. })).count();
            if shortest < 1.0 || horizontal != 0 {
                problems.push(format!("{count} line(s): Horizontal over Vertical left the shortest line {shortest:.3} long and {horizontal} Horizontal in the sketch; status {:?}", app.status));
            }
            if app.status == crate::i18n::tr("sk-constraint-added") {
                problems.push(format!("{count} line(s): the status says the constraint is added"));
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// A CONSTRAINT ALREADY HELD IS NOT LAID TWICE, AND ONE A NEW RELATION IMPLIES GOES: Vertical on three lines one of
    /// which is Vertical already adds two, not a second on that one; Vertical again on all three adds nothing and says
    /// so; Collinear on the three vertical lines takes away the Vertical it makes redundant - no constraint is left
    /// redundant at any step.
    ///
    /// Reported behaviour (found checking issue #34): a line made Vertical while it was drawn got a second Vertical with
    /// the rest, and the sketch turned yellow; Collinear on vertical lines did the same.
    #[test]
    fn a_constraint_already_held_is_not_laid_twice() {
        use qymcad_core::model::{Constraint, EntityKind};
        let (mut app, si) = a_sketch();
        for (x, lean) in [(0.0, 5.0), (20.0, 9.0), (40.0, 3.0)] {
            Hand::new(&mut app).sk_tool(1).click2d(x, 0.0).double_click2d(x + lean, 12.0);
            Hand::new(&mut app).key(egui::Key::Escape);
        }
        let lines: Vec<(u8, u64)> = app.project.sketches[si].entities.iter().filter(|e| matches!(e.kind, EntityKind::Line { .. })).map(|e| (1u8, e.id)).collect();
        let count = |app: &App, vertical: bool| {
            app.project.sketches[si].constraints.iter().filter(|c| if vertical { matches!(c, Constraint::Vertical { .. }) } else { matches!(c, Constraint::Collinear { .. }) }).count()
        };
        let mut problems = Vec::new();
        assert!(Hand::new(&mut app).select2d(&lines[..1]), "setup: the first line is picked");
        Hand::new(&mut app).constraint(2);
        assert_eq!(count(&app, true), 1, "setup: the first line is Vertical");

        assert!(Hand::new(&mut app).select2d(&lines), "setup: the three lines are picked");
        Hand::new(&mut app).constraint(2);
        let redundant = app.project.sketch_redundant_constraints(si);
        if count(&app, true) != 3 || !redundant.is_empty() {
            problems.push(format!("Vertical on three lines, one Vertical already: {} Vertical, redundant {redundant:?}; status {:?}", count(&app, true), app.status));
        }

        assert!(Hand::new(&mut app).select2d(&lines), "setup: the three lines are picked again");
        Hand::new(&mut app).constraint(2);
        if count(&app, true) != 3 || app.status != crate::i18n::tr("sk-constraint-already") {
            problems.push(format!("Vertical again on three vertical lines: {} Vertical; status {:?}", count(&app, true), app.status));
        }

        assert!(Hand::new(&mut app).select2d(&lines), "setup: the three lines are picked for Collinear");
        Hand::new(&mut app).constraint(7);
        let redundant = app.project.sketch_redundant_constraints(si);
        if count(&app, false) != 2 || count(&app, true) != 1 || !redundant.is_empty() {
            problems.push(format!("Collinear on three vertical lines: {} Collinear, {} Vertical, redundant {redundant:?}; status {:?}", count(&app, false), count(&app, true), app.status));
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// WITH A DRAWING TOOL IN HAND A DRAG NEITHER SELECTS NOR MOVES WHAT IS DRAWN: a band over two lines selects nothing,
    /// a drag from the end of a line leaves it where it is - with every drawing tool; in selection mode both work as
    /// before.
    ///
    /// Reported behaviour (issue #33): with Line in hand a drag over empty space selected the geometry in the band, and a
    /// drag from a point moved the point.
    #[test]
    fn a_drawing_tool_in_hand_takes_no_drag() {
        let lines_drawn = |app: &mut App| {
            for y in [0.0, 10.0] {
                Hand::new(app).sk_tool(1).click2d(0.0, y).double_click2d(20.0, y);
                Hand::new(app).key(egui::Key::Escape);
            }
        };
        let end_at = |app: &App, si: usize| app.project.sketches[si].points.iter().any(|p| (p.x - 20.0).abs() < 1e-6 && (p.y - 10.0).abs() < 1e-6);
        let mut problems = Vec::new();
        for tool in [1u8, 2, 3, 4, 7] {
            let (mut app, si) = a_sketch();
            lines_drawn(&mut app);
            Hand::new(&mut app).sk_tool(tool).drag2d((-5.0, -5.0), (25.0, 15.0));
            if !app.tools.sel_sk.items.is_empty() {
                problems.push(format!("tool {tool}: a band selected {} items; status {:?}", app.tools.sel_sk.items.len(), app.status));
            }
            Hand::new(&mut app).drag2d((20.0, 10.0), (25.0, 18.0));
            if !end_at(&app, si) {
                problems.push(format!("tool {tool}: a drag from the end of a line moved it"));
            }
        }
        // in selection mode the same drags still select and move
        let (mut app, si) = a_sketch();
        lines_drawn(&mut app);
        Hand::new(&mut app).sk_tool(0).drag2d((-5.0, -5.0), (25.0, 15.0));
        if app.tools.sel_sk.items.is_empty() {
            problems.push("selection mode: a band selected nothing".to_string());
        }
        Hand::new(&mut app).key(egui::Key::Escape);
        Hand::new(&mut app).drag2d((20.0, 10.0), (25.0, 18.0));
        if end_at(&app, si) {
            problems.push("selection mode: a drag from the end of a line did not move it".to_string());
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// Esc ENDS A SPLINE, AS ITS HINT SAYS, keeping the nodes that were clicked.
    #[test]
    fn escape_ends_a_spline_as_its_hint_says() {
        let (mut app, si) = a_sketch();
        let mut hand = Hand::new(&mut app);
        hand.sk_tool(9).click2d(0.0, 0.0).click2d(10.0, 10.0).click2d(20.0, 0.0);
        let hint = hand.app.status.clone();
        hand.key(egui::Key::Escape).close_window();
        assert_eq!(spline_nodes(&app, si), vec![vec![(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)]], "Esc on a spline of three nodes, under the hint {hint:?}");
    }

    /// A drawing tool by its number, the key of its name, and the clicks that draw with it.
    struct Drawing {
        tool: u8,
        key: &'static str,
        clicks: &'static [(f64, f64)],
    }

    fn drawing(tool: u8, key: &'static str, clicks: &'static [(f64, f64)]) -> Drawing {
        Drawing { tool, key, clicks }
    }

    /// EVERY DRAWING TOOL NAMES ITS STEP OF UNDO AFTER WHAT IT DREW.
    ///
    /// Failures are gathered and given out together.
    #[test]
    fn every_drawing_tool_names_its_step_of_undo() {
        let tools: [Drawing; 11] = [
            drawing(1, "sk-line", &[(0.0, 0.0), (20.0, 0.0)]),
            drawing(2, "sk-rect", &[(0.0, 0.0), (20.0, 15.0)]),
            drawing(3, "sk-circle", &[(0.0, 0.0), (8.0, 0.0)]),
            drawing(4, "sk-arc", &[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)]),
            drawing(5, "sk-point", &[(5.0, 5.0)]),
            drawing(6, "sk-polygon", &[(0.0, 0.0), (8.0, 0.0)]),
            drawing(7, "sk-slot", &[(0.0, 0.0), (15.0, 0.0), (15.0, 5.0)]),
            drawing(8, "sk-ellipse", &[(0.0, 0.0), (12.0, 0.0), (6.0, 7.0)]),
            drawing(9, "sk-spline", &[(0.0, 0.0), (7.0, 7.0), (15.0, 0.0)]),
            drawing(10, "sk-circle", &[(0.0, 0.0), (8.0, 5.0), (12.0, -3.0)]),
            drawing(11, "sk-text", &[(0.0, 0.0)]),
        ];
        let mut problems = Vec::new();
        for Drawing { tool, key, clicks } in tools {
            let (mut app, _si) = a_sketch();
            let mut hand = Hand::new(&mut app);
            if tool == 11 {
                hand.sk_text("CAD", 5.0);
            } else {
                hand.sk_tool(tool);
            }
            let (last, before) = clicks.split_last().expect("every tool takes a click");
            for (x, y) in before {
                hand.click2d(*x, *y);
            }
            if tool == 9 {
                hand.double_click2d(last.0, last.1);
            } else {
                hand.click2d(last.0, last.1);
            }
            let step = app.disk.edits.undo.last().map(|s| s.name.clone());
            if step.as_deref() != Some(crate::i18n::tr(key).as_str()) {
                problems.push(format!("tool {tool}: the step of undo is {step:?}, it should be {:?}", crate::i18n::tr(key)));
            }
        }
        assert!(problems.is_empty(), "a drawing is undone under another name:\n{}", problems.join("\n"));
    }
}
