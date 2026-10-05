//! A SKETCH RECTANGLE IS ONE SHAPE, as a circle is: in every way it is drawn it is held by its sides - parallel in
//! pairs, square to each other - and a turn of its own, not by the axes; it has a centre where its construction
//! diagonals meet; a dragged corner changes its size and not its turn; and a side deleted breaks it into four plain
//! lines, its centre and diagonals gone with it.
//!
//! Reported (issue #56): a rectangle was four lines held Horizontal and Vertical, so it could not be turned without
//! deleting those constraints by hand, and a rectangle drawn from its centre kept no centre.
use qymcad_core::feature::Purpose;
use qymcad_core::geom::Point2;
use qymcad_core::model::{Constraint, EntityKind, Project};

/// The three ways a rectangle is drawn: two corners, a centre and a corner, three points (turned by 30 deg).
fn drawn(way: usize) -> (Project, usize) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    match way {
        0 => drop(p.add_rect_entity(si, 10.0, 10.0, 50.0, 40.0, Purpose::Real)),
        1 => drop(p.add_rect_from_centre(si, Point2::new(30.0, 25.0), Point2::new(50.0, 40.0), Purpose::Real)),
        _ => drop(p.add_rect3_entity(si, Point2::new(10.0, 10.0), Point2::new(10.0 + 40.0 * c, 10.0 + 40.0 * s), Point2::new(10.0 + 40.0 * c - 30.0 * s, 10.0 + 40.0 * s + 30.0 * c), Purpose::Real)),
    }
    p.solve_sketch(si);
    (p, si)
}

fn xy(p: &Project, si: usize, id: u64) -> (f64, f64) {
    let q = p.sketches[si].points.iter().find(|q| q.id == id).expect("a point of the rectangle");
    (q.x, q.y)
}

/// The turn of the first side, the length of the first and the second side, and how far the corners stand from a
/// right angle (the dot product of the two sides at the first corner, over their lengths).
struct Shape {
    turn: f64,
    width: f64,
    height: f64,
    off_square: f64,
}

fn shape(p: &Project, si: usize) -> Shape {
    let r = &p.sketches[si].rects[0];
    let [a, b, _, d] = r.corners.map(|id| xy(p, si, id));
    let (u, v) = ((b.0 - a.0, b.1 - a.1), (d.0 - a.0, d.1 - a.1));
    let (lu, lv) = (u.0.hypot(u.1), v.0.hypot(v.1));
    Shape { turn: u.1.atan2(u.0).to_degrees(), width: lu, height: lv, off_square: (u.0 * v.0 + u.1 * v.1) / (lu * lv) }
}

#[test]
fn a_rectangle_is_held_by_its_shape_and_keeps_a_centre() {
    let mut sins = Vec::new();
    for (way, turn) in [(0, 0.0), (1, 0.0), (2, 30.0)] {
        let (p, si) = drawn(way);
        let s = &p.sketches[si];
        if s.rects.len() != 1 {
            sins.push(format!("way {way}: {} rectangle records, not one", s.rects.len()));
            continue;
        }
        let r = &s.rects[0];
        // held by its shape and its turn, not by the axes
        if s.constraints.iter().any(|c| matches!(c, Constraint::Horizontal { .. } | Constraint::Vertical { .. })) {
            sins.push(format!("way {way}: the rectangle is still held Horizontal or Vertical"));
        }
        if !s.constraints.iter().any(|c| matches!(c, Constraint::Orientation { deg, .. } if (deg - turn).abs() < 1e-9)) {
            sins.push(format!("way {way}: no turn of its own held at {turn} deg"));
        }
        // four sides; two construction diagonals for a rectangle drawn from its centre, none for one drawn by its corners;
        // the centre on the middle of both diagonals all the same
        let lines = |construction: bool| s.entities.iter().filter(|e| e.construction == construction && matches!(e.kind, EntityKind::Line { .. })).count();
        let diagonals = if way == 1 { 2 } else { 0 };
        if lines(false) != 4 || lines(true) != diagonals {
            sins.push(format!("way {way}: {} sides and {} diagonals, not 4 and {diagonals}", lines(false), lines(true)));
        }
        let [a, b, c, d] = r.corners.map(|id| xy(&p, si, id));
        let m = xy(&p, si, r.centre);
        if (m.0 - (a.0 + c.0) / 2.0).hypot(m.1 - (a.1 + c.1) / 2.0) > 1e-9 || (m.0 - (b.0 + d.0) / 2.0).hypot(m.1 - (b.1 + d.1) / 2.0) > 1e-9 {
            sins.push(format!("way {way}: the centre {m:?} is not the middle of the diagonals"));
        }
        // a rectangle in an empty sketch is left with its place, its width and its height
        if p.sketch_dof(si) != (4, 0) {
            sins.push(format!("way {way}: (freedoms, redundant) = {:?}, not (4, 0)", p.sketch_dof(si)));
        }
    }
    // the centre of a rectangle drawn from the centre is where the centre was clicked
    let (p, si) = drawn(1);
    let m = xy(&p, si, p.sketches[si].rects[0].centre);
    if (m.0 - 30.0).hypot(m.1 - 25.0) > 1e-9 {
        sins.push(format!("drawn from (30, 25): the centre stands at {m:?}"));
    }
    assert!(sins.is_empty(), "{}", sins.join("\n"));
}

#[test]
fn a_dragged_corner_changes_the_size_and_not_the_turn() {
    let mut sins = Vec::new();
    for way in [0, 2] {
        let (mut p, si) = drawn(way);
        let before = shape(&p, si);
        let corner = p.sketches[si].rects[0].corners[2];
        let (x, y) = xy(&p, si, corner);
        p.solve_sketch_drag(si, Some((corner, x + 7.0, y + 3.0)));
        p.solve_sketch(si);
        let after = shape(&p, si);
        if (after.turn - before.turn).abs() > 1e-6 || after.off_square.abs() > 1e-6 {
            sins.push(format!("way {way}: the drag turned the rectangle from {:.4} to {:.4} deg (off square {:.2e})", before.turn, after.turn, after.off_square));
        }
        if (after.width - before.width).abs() < 1e-3 && (after.height - before.height).abs() < 1e-3 {
            sins.push(format!("way {way}: the drag changed nothing"));
        }
    }
    assert!(sins.is_empty(), "{}", sins.join("\n"));
}

#[test]
fn a_rectangle_losing_a_side_is_four_plain_lines() {
    let (mut p, si) = drawn(1);
    let side = p.sketches[si].rects[0].sides[0];
    let centre = p.sketches[si].rects[0].centre;
    p.delete_entities(si, &[side]);
    let s = &p.sketches[si];
    let mut sins = Vec::new();
    if !s.rects.is_empty() {
        sins.push("the rectangle record stayed".to_string());
    }
    if s.entities.iter().any(|e| e.construction) {
        sins.push("a diagonal stayed".to_string());
    }
    if s.points.iter().any(|q| q.id == centre) {
        sins.push("the centre stayed".to_string());
    }
    if let Some(c) = s.constraints.iter().find(|c| matches!(c, Constraint::Parallel { .. } | Constraint::Perpendicular { .. } | Constraint::Orientation { .. } | Constraint::Midpoint { .. })) {
        sins.push(format!("a constraint of the rectangle stayed: {c:?}"));
    }
    if s.entities.iter().filter(|e| matches!(e.kind, EntityKind::Line { .. })).count() != 3 {
        sins.push("the other three sides did not stay".to_string());
    }
    assert!(sins.is_empty(), "{}", sins.join("\n"));
}

#[test]
fn a_corner_of_a_rectangle_is_still_rounded() {
    let (mut p, si) = drawn(0);
    let corner = p.sketches[si].rects[0].corners[0];
    assert!(p.fillet_at_vertex(si, corner, 3.0), "the corner of a rectangle with its diagonals refused a fillet");
    let resid = p.solve_sketch(si);
    assert!(resid < 1e-6, "the rounded rectangle is unsolved: {resid:e}");
    assert!(p.sketches[si].rects.len() == 1, "rounding a corner broke the rectangle");
}

/// A RECTANGLE CHANGES ITS SIZE FROM WHERE IT WAS DRAWN: drawn from the centre, the centre stays when a corner is
/// dragged and when the width dimension is changed; drawn by its corners, the first corner stays.
#[test]
fn a_rectangle_grows_from_where_it_was_drawn() {
    let mut sins = Vec::new();
    // drawn from the centre: a corner dragged
    let (mut p, si) = drawn(1);
    let r = p.sketches[si].rects[0].clone();
    let (x, y) = xy(&p, si, r.corners[2]);
    p.solve_sketch_drag(si, Some((r.corners[2], x + 6.0, y + 4.0)));
    p.solve_sketch(si);
    let m = xy(&p, si, r.centre);
    if (m.0 - 30.0).hypot(m.1 - 25.0) > 1e-6 {
        sins.push(format!("drawn from the centre, a corner dragged: the centre went from (30, 25) to {m:?}"));
    }
    // a width dimension changed, on a rectangle drawn from the centre and on one drawn by its corners
    for (way, keep) in [(1, "the centre"), (0, "the first corner")] {
        let (mut p, si) = drawn(way);
        let r = p.sketches[si].rects[0].clone();
        let kept = if way == 1 { r.centre } else { r.corners[0] };
        let before = xy(&p, si, kept);
        let w = shape(&p, si).width;
        p.sketches[si].constraints.push(Constraint::Distance { a: r.corners[0], b: r.corners[1], d: w, off: 0.0, expr: String::new(), driven: false, axis: 0, at: None });
        p.solve_sketch(si);
        let ci = p.sketches[si].constraints.len() - 1;
        if let Constraint::Distance { d, .. } = &mut p.sketches[si].constraints[ci] {
            *d = w + 10.0;
        }
        let resid = p.solve_sketch(si);
        let after = xy(&p, si, kept);
        let now = shape(&p, si);
        if resid > 1e-6 || (now.width - w - 10.0).abs() > 1e-6 {
            sins.push(format!("way {way}: the width {w} made {} came out {} (residual {resid:e})", w + 10.0, now.width));
        }
        if (after.0 - before.0).hypot(after.1 - before.1) > 1e-6 {
            sins.push(format!("way {way}: the width changed moved {keep} from {before:?} to {after:?}"));
        }
    }
    assert!(sins.is_empty(), "{}", sins.join("\n"));
}
