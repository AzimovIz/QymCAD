//! A CORNER HELD BY THE AUTOMATIC CONSTRAINTS A PERSON'S DEFAULTS PUT ON IT - the one line horizontal, the other
//! vertical, the two equal - rounds and bevels as a bare one: the constraints go with the shortened lines, and the
//! vanished corner does not stay behind as a point of its own. A leg or a radius past what the corner takes is refused.
//!
//! Reported behaviour: with the automatic constraints on, the typed radius did nothing - no arc, no step of undo, no word.
use qymcad_core::feature::Purpose;
use qymcad_core::model::{Constraint, EntityKind, Project};

/// The corner (30, 0) -> (0, 0) -> (0, 30) with its automatic constraints; (project, sketch, the corner point).
fn corner() -> (Project, usize, u64) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let l1 = p.add_line_entity(si, 30.0, 0.0, 0.0, 0.0, Purpose::Real);
    let l2 = p.add_line_entity(si, 0.0, 0.0, 0.0, 30.0, Purpose::Real);
    let ends = |p: &Project, id| match p.sketches[si].entities.iter().find(|e| e.id == id).map(|e| e.kind) {
        Some(EntityKind::Line { a, b }) => (a, b),
        _ => panic!("a line"),
    };
    let ((a1, b1), (a2, b2)) = (ends(&p, l1), ends(&p, l2));
    p.sketches[si].constraints.extend([Constraint::Horizontal { a: a1, b: b1 }, Constraint::Vertical { a: a2, b: b2 }, Constraint::Equal { a: a1, b: b1, c: a2, d: b2 }]);
    p.regen_sketch(si);
    (p, si, b1)
}

fn counts(p: &Project, si: usize) -> (usize, usize, usize) {
    let s = &p.sketches[si];
    let lines = s.entities.iter().filter(|e| matches!(e.kind, EntityKind::Line { .. })).count();
    let arcs = s.entities.iter().filter(|e| matches!(e.kind, EntityKind::Arc { .. })).count();
    let system = s.system_ids();
    (s.points.iter().filter(|q| !system.contains(&q.id)).count(), lines, arcs)
}

#[test]
fn a_corner_with_its_automatic_constraints_rounds_and_bevels() {
    let (mut p, si, pc) = corner();
    assert!(p.fillet_at_vertex(si, pc, 3.0), "the corner rounded R3");
    assert_eq!(counts(&p, si), (5, 2, 1), "the far ends, the two points of touching and the centre - the corner gone");
    let (mut p, si, pc) = corner();
    assert!(p.chamfer_at_vertex(si, pc, 3.0), "the corner bevelled 3");
    assert_eq!(counts(&p, si), (4, 3, 0), "the far ends and the two legs - the corner gone");
}

#[test]
fn a_corner_refuses_what_it_cannot_take() {
    let (p, si, pc) = corner();
    let (r, d) = (p.corner_limit(si, pc, false), p.corner_limit(si, pc, true));
    assert!(r.is_some_and(|r| (r - 30.0).abs() < 1e-9) && d.is_some_and(|d| (d - 30.0).abs() < 1e-9), "a square corner of lines 30 takes up to 30: {r:?} {d:?}");
    let (mut p, si, pc) = corner();
    assert!(!p.chamfer_at_vertex(si, pc, 300.0), "a leg of 300 on lines of 30 is refused, not cut down");
    let (mut p, si, pc) = corner();
    assert!(p.chamfer_at_vertex(si, pc, 29.9), "a leg of 29.9 is taken as it is");
    let legs: Vec<(f64, f64)> = p.sketches[si].points.iter().map(|q| (q.x, q.y)).collect();
    assert!(legs.iter().any(|&(x, y)| (x - 29.9).abs() < 1e-6 && y.abs() < 1e-6), "the leg stands at 29.9, not pressed down: {legs:?}");
}
