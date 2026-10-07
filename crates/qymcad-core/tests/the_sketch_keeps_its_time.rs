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
    // The solve of a drag frame alone: the frame through `Project::solve_sketch_drag_fast` also rebuilds the contours
    // of the sketch, which intersect every curve with every other - 14 s among 10 000 lines, a step of its own.
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
