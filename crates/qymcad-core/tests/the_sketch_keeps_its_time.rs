//! A SKETCH KEEPS ITS TIME: the budgets of issue #95 on sketches of separate shapes (`sketch_kinds`), timed through the
//! project's own doors. The measure of every kind and size is `sketch_speed.rs`; these are the budgets that hold.
mod sketch_kinds;

use qymcad_core::solver;
use sketch_kinds::{build, Kind};
use std::time::{Duration, Instant};

/// The time of `work`, the best of three: a check of a budget is a check of the work, not of what else the machine did.
fn best_of_three(mut work: impl FnMut()) -> Duration {
    (0..3)
        .map(|_| {
            let started = Instant::now();
            work();
            started.elapsed()
        })
        .min()
        .unwrap_or_default()
}

#[test]
fn separate_shapes_are_solved_each_alone() {
    // Reported behaviour: 300 arcs, each with a Horizontal not yet met, took 6.8 s to solve, 600 arcs 27 s.
    for kind in [Kind::Lines, Kind::Arcs] {
        let p = build(kind, 300);
        let t = best_of_three(|| {
            let _ = p.clone().solve_sketch(0);
        });
        eprintln!("{kind:?} x300, solve: {t:?}");
        assert!(t < Duration::from_millis(500), "300 separate {kind:?} took {t:?} to solve, budget 500 ms");
    }
}

#[test]
fn a_drag_frame_solves_its_own_shape_alone() {
    // The solve of a drag frame alone; the whole frame, with the contours rebuilt, is
    // `a_drag_frame_among_separate_shapes_stays_in_its_frame`.
    let mut p = build(Kind::Lines, 10_000);
    p.solve_sketch(0);
    let s = &mut p.sketches[0];
    let (q, constraints) = (s.points[0], s.constraints.clone());
    let drag = Some((q.id, q.x + 1.0, q.y + 1.0));
    let t = best_of_three(|| {
        let _ = solver::solve_full_iter(&mut s.points, &mut Vec::new(), &constraints, drag, 40);
    });
    eprintln!("Lines x10000, the solve of a drag frame: {t:?}");
    assert!(t < Duration::from_millis(16), "the solve of a drag frame among 10 000 lines took {t:?}, budget 16 ms");
}

#[test]
fn an_array_of_rectangles_is_solved_as_one_sparse_part() {
    // 300 rectangles tied by Equal and spacing dimensions: one part of 2 400 unknowns, whose dense step is 1.4e10
    // operations and a matrix of 46 MB
    let p = build(Kind::Array, 300);
    let mut residual = f64::INFINITY;
    let t = best_of_three(|| residual = p.clone().solve_sketch(0));
    assert!(residual < 1e-6, "the array is not solved: residual {residual:e}");
    eprintln!("Array x300, solve: {t:?}");
    assert!(t < Duration::from_millis(2_000), "an array of 300 rectangles took {t:?} to solve, budget 2 s");
}

#[test]
fn the_diagnostics_of_separate_shapes_are_counted_each_alone() {
    // Measured before: the degrees of freedom of 10 000 separate lines 49 s, the conflicts 6 s, the redundant ones 49 s.
    let mut p = build(Kind::Lines, 10_000);
    // every line laid Horizontal twice, so the redundant ones are looked for in every part
    let twice: Vec<_> = p.sketches[0].constraints.clone();
    p.sketches[0].constraints.extend(twice);
    let t = best_of_three(|| {
        let _ = (p.sketch_dof(0), p.sketch_free_points(0), p.sketch_conflicts(0));
    });
    eprintln!("Lines x10000, degrees of freedom, free points, conflicts: {t:?}");
    assert!(t < Duration::from_secs(1), "the diagnostics of 10 000 lines took {t:?}, budget 1 s");
    let started = Instant::now();
    let redundant = p.sketch_redundant_constraints(0);
    let t = started.elapsed();
    eprintln!("Lines x10000, redundant: {t:?}");
    assert_eq!(redundant.len(), 20_000, "each Horizontal of a line laid twice is redundant");
    assert!(t < Duration::from_secs(1), "the redundant constraints of 10 000 lines took {t:?}, budget 1 s");
}

#[test]
fn the_contours_of_separate_shapes_are_built_without_a_pass_of_all_pairs() {
    // Measured before: the contours of 10 000 separate lines 3 s in a release build, 14 s in a test build - every
    // curve was intersected with every other.
    for kind in [Kind::Lines, Kind::Rectangles, Kind::Circles] {
        let mut p = build(kind, 10_000);
        let t = best_of_three(|| p.regen_sketch(0));
        eprintln!("{kind:?} x10000, contours: {t:?}");
        assert!(t < Duration::from_secs(1), "the contours of 10 000 {kind:?} took {t:?}, budget 1 s");
    }
}

#[test]
fn a_drag_frame_among_separate_shapes_stays_in_its_frame() {
    // the whole frame through the project's door: the solve of the dragged part and the contours rebuilt
    let mut p = build(Kind::Lines, 10_000);
    p.solve_sketch(0);
    let q = p.sketches[0].points[0];
    let t = best_of_three(|| {
        let _ = p.solve_sketch_drag_fast(0, Some((q.id, q.x + 1.0, q.y + 1.0)));
    });
    eprintln!("Lines x10000, drag frame: {t:?}");
    assert!(t < Duration::from_millis(100), "a drag frame among 10 000 lines took {t:?}, budget 100 ms in a test build");
}

#[test]
fn a_constraint_laid_among_separate_shapes_is_judged_in_its_own() {
    // Reported behaviour (#95): a constraint or a dimension laid in a sketch of 300 lines took seconds to minutes.
    let p = build(Kind::Lines, 10_000);
    let s = &p.sketches[0];
    // a Vertical between the first point of the first line and the last point of the last: two parts made one
    let (a, b) = (s.points[0].id, s.points[s.points.len() - 1].id);
    let mut laid = false;
    let t = best_of_three(|| laid = p.clone().add_constraint_if_independent(0, qymcad_core::model::Constraint::Vertical { a, b }));
    eprintln!("Lines x10000, a constraint laid: {t:?}");
    assert!(laid, "the Vertical constrains something and is laid");
    assert!(t < Duration::from_millis(500), "a constraint laid among 10 000 lines took {t:?}, budget 500 ms in a test build");
}

#[test]
fn separate_circles_and_tangents_are_solved_each_alone() {
    // Measured in a release build with the solve by parts: 70 000 circles 12.8 s to solve and 7.9 s a drag frame, 70 000
    // tangent lines and circles 43 s - the solved radii went back into the circles as every radius against every entity.
    for kind in [Kind::Circles, Kind::Tangents] {
        let p = build(kind, 10_000);
        let t = best_of_three(|| {
            let _ = p.clone().solve_sketch(0);
        });
        eprintln!("{kind:?} x10000, solve: {t:?}");
        assert!(t < Duration::from_secs(2), "10 000 {kind:?} took {t:?} to solve, budget 2 s in a test build");
    }
}

#[test]
fn the_diagnostics_of_one_big_part_are_counted_sparse() {
    // 1 000 rectangles tied into one part of 8 000 unknowns; measured in a release build with the dense count: the
    // degrees of freedom 16 s, the free points 18 s, the redundant constraints 16 s, the conflicts 1 s (12 s on 3 000)
    let p = build(Kind::Array, 1_000);
    let started = Instant::now();
    let dof = p.sketch_dof(0);
    let free = p.sketch_free_points(0);
    let conflicts = p.sketch_conflicts(0);
    let t = started.elapsed();
    eprintln!("Array x1000, degrees of freedom, free points and conflicts: {t:?}");
    assert!(conflicts.is_empty(), "an array that can be solved has no conflicts: {conflicts:?}");
    // each rectangle keeps its height and its place up and down free (the spacing is dimensioned along the row only),
    // the first its place along the row too
    assert_eq!(dof, (2 * 1_000 + 1, 0), "the degrees of freedom of the array");
    assert!(free.iter().any(|f| *f), "an array that is free to move has free points");
    assert!(t < Duration::from_secs(2), "the diagnostics of an array of 1 000 rectangles took {t:?}, budget 2 s in a test build");
}
