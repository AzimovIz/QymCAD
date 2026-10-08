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

#[test]
fn a_drawing_of_forty_thousand_segments_comes_in_whole() {
    // Measured through the window in a release build: 70 000 segments took 5.3 s to lay on a plane - every end was
    // looked for among every point made before it.
    use qymcad_core::geom::{Point2, ProfEdge};
    let n = 10_000;
    let side = (n as f64).sqrt().ceil() as usize;
    let curves: Vec<ProfEdge> = (0..n)
        .flat_map(|k| {
            let (x, y) = ((k % side) as f64 * 20.0, (k / side) as f64 * 20.0);
            let c = [Point2::new(x, y), Point2::new(x + 10.0, y), Point2::new(x + 10.0, y + 6.0), Point2::new(x, y + 6.0)];
            (0..4).map(move |i| ProfEdge::Line { a: c[i], b: c[(i + 1) % 4] })
        })
        .collect();
    let mut p = qymcad_core::model::Project::default();
    p.new_document();
    let started = Instant::now();
    let si = p.import_sketch("drawing", curves, None, Default::default());
    let t = started.elapsed();
    eprintln!("40 000 segments imported: {t:?}");
    let s = &p.sketches[si];
    let own = s.points.len() - s.system_ids().len();
    assert_eq!(own, 4 * n, "every rectangle is four points, its corners shared by its sides");
    assert_eq!(s.entities.len(), 4 * n);
    assert!(t < Duration::from_secs(3), "40 000 segments took {t:?} to come in, budget 3 s in a test build");
}

#[test]
fn rectangles_drawn_by_their_corners_are_solved_without_a_pass_of_all_pairs() {
    // Every rectangle drawn by its corners asked every constraint of the sketch whether it named its centre, on every
    // solve: 2 000 rectangles, 12 000 constraints, 2.4e7 asks a solve - 0.39 s in a test build, 55 ms counted once.
    let mut p = qymcad_core::model::Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    for k in 0..2_000 {
        let (x, y) = ((k % 50) as f64 * 20.0, (k / 50) as f64 * 20.0);
        p.add_rect_entity(si, x, y, x + 10.0, y + 6.0, qymcad_core::feature::Purpose::Real);
    }
    let t = best_of_three(|| {
        let _ = p.clone().solve_sketch(si);
    });
    eprintln!("2 000 rectangles drawn by their corners, solve: {t:?}");
    assert!(t < Duration::from_millis(200), "2 000 rectangles drawn by their corners took {t:?} to solve, budget 200 ms in a test build");
}

#[test]
fn a_drag_frame_after_the_first_works_on_its_own_part() {
    // A release build takes the sketch of the bar, 70 000 rectangles, and holds a frame to 16 ms; a test build, some ten
    // times slower, takes 10 000. Measured in a release build on 70 000 rectangles: 1.76 s a frame rebuilt whole, 0.48 s
    // with the loops of what moved alone, 35 ms with the drag remembering its part, 11 ms with the loops looked up on
    // grids and the points of the group alone. In a test build on 10 000: 59 ms without the session, 1.5 ms with it.
    let release = !cfg!(debug_assertions);
    let (n, budget) = if release { (70_000, Duration::from_millis(16)) } else { (10_000, Duration::from_millis(15)) };
    let mut p = build(Kind::Rectangles, n);
    p.solve_sketch(0);
    let q = p.sketches[0].points[0];
    let _ = p.solve_sketch_drag_fast(0, Some((q.id, q.x + 0.5, q.y + 0.5))); // the first frame finds the part
    let t = (1..=5)
        .map(|k| {
            let started = Instant::now();
            let _ = p.solve_sketch_drag_fast(0, Some((q.id, q.x + k as f64, q.y + k as f64)));
            started.elapsed()
        })
        .min()
        .unwrap_or_default();
    eprintln!("Rectangles x{n}, a drag frame after the first: {t:?}");
    assert!(t < budget, "a drag frame after the first among {n} rectangles took {t:?}, budget {budget:?}");
}

#[test]
fn a_pattern_across_a_pattern_lays_its_copies_with_one_rebuild() {
    // Reported behaviour: a line and another across it, each patterned 200 times 20 mm apart - a grid of 40 000 cells
    // - and every operation of the sketch waits 30 s. Each copy of a pattern rebuilt the loops of the growing grid: the
    // second pattern took 29 s in a release build. 100 by 100 here, a test build: 3 s rebuilt after each copy, 56 ms
    // rebuilt once.
    use qymcad_core::model::PatternKind;
    let mut p = qymcad_core::model::Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let h = p.add_line_entity(si, 0.0, -10.0, 2000.0, -10.0, qymcad_core::feature::Purpose::Real);
    let v = p.add_line_entity(si, -10.0, 0.0, -10.0, 2000.0, qymcad_core::feature::Purpose::Real);
    p.add_pattern(si, &[h], PatternKind::Linear { dx: 0.0, dy: 20.0, count: 100, dx2: 0.0, dy2: 0.0, count2: 0 });
    let started = Instant::now();
    p.add_pattern(si, &[v], PatternKind::Linear { dx: 20.0, dy: 0.0, count: 100, dx2: 0.0, dy2: 0.0, count2: 0 });
    let t = started.elapsed();
    eprintln!("a pattern of 100 lines across 100 others: {t:?}, {} loops", p.sketches[si].contour_ids.len());
    assert!(p.sketches[si].contour_ids.len() > 9_000, "the grid has its cells: {} loops", p.sketches[si].contour_ids.len());
    assert!(t < Duration::from_millis(500), "a pattern of 100 lines across 100 others took {t:?}, budget 0.5 s in a test build");
}

#[test]
fn the_checks_after_an_edit_count_the_part_it_touched() {
    // 17 500 rectangles - 70 000 segments - their checks counted once, then a rectangle moved and the checks counted
    // again, three times on three rectangles: the quickest, as a test run beside others is slowed by them
    let mut p = sketch_kinds::build(sketch_kinds::Kind::Rectangles, 17_500);
    p.solve_sketch(0);
    let _ = (p.sketch_checks(0), p.sketch_conflicts(0));
    let mut times = Vec::new();
    for k in [0, 4_000, 40_000] {
        if let Some(pt) = p.sketches[0].points.get_mut(k) {
            pt.x += 0.5;
        }
        let started = std::time::Instant::now();
        let _ = (p.sketch_checks(0), p.sketch_conflicts(0));
        times.push(started.elapsed());
    }
    let took = times.iter().min().copied().unwrap_or_default();
    eprintln!("the checks of 17 500 rectangles after one moved: {times:?}");
    // measured in a release build: 225 ms with every part counted again on each change, 92 ms with the parts remembered
    // (120 ms beside the other checks of a release run)
    let budget = std::time::Duration::from_millis(150);
    assert!(took < budget, "the checks of 17 500 rectangles after one moved took {took:?}, budget {budget:?}");
}
