//! THE POINT OF A CORNER GOES WITH THE CORNER, and what was stated about it goes with the point.
//!
//! The rule, asked for and now kept in one place: after a fillet or a chamfer, if no line comes to the point of the
//! corner any more, the point is deleted. A point of the sketch that draws nothing is not a leftover to be cleaned
//! by hand — the picking offers it and the solver counts it, so it answers "here is a corner" where there is none.
//!
//! And the point stays exactly when the rest of the drawing still needs it: a third line at a T, or the two lines of
//! the far square at the point two squares share.
use qymcad_core::feature::Purpose;
use qymcad_core::model::{ChamferLegs, Constraint, EntityKind, Project};

/// An L of two lines meeting at (20, 0), with the point of the corner and the far end of the bottom line.
fn an_angle() -> (Project, usize, u64, u64, u64) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    p.add_line_entity(si, 0.0, 0.0, 20.0, 0.0, Purpose::Real);
    p.add_line_entity(si, 20.0, 0.0, 20.0, 20.0, Purpose::Real);
    p.regen_sketch(si);
    let corner = p.sketches[si].points.iter().find(|q| (q.x - 20.0).abs() < 1e-6 && q.y.abs() < 1e-6).map(|q| q.id).expect("the corner");
    let far = p.sketches[si].points.iter().find(|q| q.x.abs() < 1e-6 && q.y.abs() < 1e-6).map(|q| q.id).expect("the far end");
    (p, si, corner, far, 0)
}

/// The points of the sketch that no entity and no spline stands on.
fn loose_points(p: &Project, si: usize) -> Vec<(f64, f64)> {
    let used: std::collections::HashSet<u64> = p.sketches[si].entities.iter().flat_map(entity_points).collect();
    p.sketches[si].points.iter().filter(|q| !used.contains(&q.id)).map(|q| (q.x, q.y)).collect()
}

/// The ids an entity stands on: the ends of a line, the centre and the ends of an arc.
fn entity_points(e: &qymcad_core::model::SketchEntity) -> Vec<u64> {
    match e.kind {
        EntityKind::Line { a, b } => vec![a, b],
        EntityKind::Arc { center, a, b, .. } => vec![center, a, b],
        EntityKind::Circle { center, .. } => vec![center],
        EntityKind::Ellipse { c, ma, mi } => vec![c, ma, mi],
    }
}

#[test]
fn a_chamfered_corner_leaves_no_point_behind() {
    let (mut p, si, corner, _far, _) = an_angle();
    assert!(p.chamfer_at_vertex(si, corner, ChamferLegs::equal(5.0), None), "the corner was not cut");
    assert!(!p.sketches[si].points.iter().any(|q| q.id == corner), "the point of a corner nothing stands on any more stayed in the sketch");
    assert!(loose_points(&p, si).is_empty(), "a point drawing nothing is left behind: {:?}", loose_points(&p, si));
}

#[test]
fn a_dimension_on_the_corner_goes_with_the_point() {
    for fillet in [false, true] {
        let (mut p, si, corner, far, _) = an_angle();
        p.sketches[si].constraints.push(Constraint::Distance { a: corner, b: far, d: 20.0, off: 0.0, expr: String::new(), driven: false, axis: 0, at: None });
        p.solve_sketch(si);
        assert_eq!(p.constraints_on_point(si, corner), 1, "setup: the dimension is measured to the corner");

        if fillet {
            assert!(p.fillet_at_vertex(si, corner, 5.0), "the corner was not rounded");
        } else {
            assert!(p.chamfer_at_vertex(si, corner, ChamferLegs::equal(5.0), None), "the corner was not cut");
        }
        let what = if fillet { "rounding" } else { "the chamfer" };
        assert!(!p.sketches[si].points.iter().any(|q| q.id == corner), "{what}: the point of the corner stayed although no line comes to it");
        assert_eq!(p.constraints_on_point(si, corner), 0, "{what}: a dimension is left standing on a point that is not there");
        assert!(!p.sketches[si].constraints.iter().any(|c| matches!(c, Constraint::Distance { a, .. } if *a == corner)), "{what}: the dimension measured to the corner was left behind, and the solver would chase an id that is gone");
        assert!(loose_points(&p, si).is_empty(), "{what}: a point drawing nothing is left behind: {:?}", loose_points(&p, si));
    }
}

#[test]
fn the_point_stays_while_a_line_still_comes_to_it() {
    // a T: three lines meet at (20, 0), and the corner is taken off two of them
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    p.add_line_entity(si, 0.0, 0.0, 20.0, 0.0, Purpose::Real);
    p.add_line_entity(si, 20.0, 0.0, 20.0, 20.0, Purpose::Real);
    p.add_line_entity(si, 20.0, 0.0, 40.0, 0.0, Purpose::Real);
    p.regen_sketch(si);
    let corner = p.sketches[si].points.iter().find(|q| (q.x - 20.0).abs() < 1e-6 && q.y.abs() < 1e-6).map(|q| q.id).expect("the point of the T");
    let towards = |p: &Project, to: (f64, f64)| -> u64 {
        p.vertex_edges(si, corner)
            .into_iter()
            .find(|&e| {
                let Some(EntityKind::Line { a, b }) = p.sketches[si].entities.iter().find(|x| x.id == e).map(|x| x.kind.clone()) else { return false };
                let other = if a == corner { b } else { a };
                p.sketches[si].points.iter().any(|q| q.id == other && (q.x - to.0).abs() < 1e-6 && (q.y - to.1).abs() < 1e-6)
            })
            .unwrap_or_else(|| panic!("no line at the point of the T runs to {to:?}"))
    };
    let (up, left) = (towards(&p, (20.0, 20.0)), towards(&p, (0.0, 0.0)));
    assert!(p.chamfer_lines_of_pair(si, (up, left), ChamferLegs::equal(5.0), None), "the corner of the T was not cut");
    assert!(p.sketches[si].points.iter().any(|q| q.id == corner), "the point of the T went although the third line still comes to it");
    assert!(loose_points(&p, si).is_empty(), "a point drawing nothing is left behind: {:?}", loose_points(&p, si));
}