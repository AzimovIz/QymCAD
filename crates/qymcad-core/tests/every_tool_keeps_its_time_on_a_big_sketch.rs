//! EVERY TOOL OF A SKETCH KEEPS ITS TIME ON A BIG SKETCH: each tool, through the door of the project the window calls,
//! on sketches of thousands of shapes - separate rectangles, a grid of two patterns, a pattern tied into one part, a
//! mixture - takes no more than a few rebuilds of that sketch. A tool that rebuilds the whole sketch for every element
//! it lays takes as many rebuilds as elements, and is caught however fast the machine.
//!
//! Reported behaviour (#95): a line and one across it, each patterned 200 times, and every operation waited 30 s - the
//! pattern rebuilt the sketch for each of its copies. The measure of the solve and the checks had not looked at the
//! tools that change a sketch.
mod sketch_kinds;

use qymcad_core::geom::Point2;
use qymcad_core::model::{ChamferLegs, Constraint, CornerAt, CornerCut, EntityKind, FilletSize, Id, PatternKind, Project};
use sketch_kinds::{build, Kind};
use std::time::{Duration, Instant};

/// A big sketch of one kind, the sketch being the first of the project.
struct Big {
    name: &'static str,
    project: Project,
}

/// The grid of the report: a horizontal line and a vertical one, each patterned `n` times 20 mm apart.
fn grid(n: u32) -> Project {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let len = 20.0 * n as f64;
    let h = p.add_line_entity(si, 0.0, -10.0, len, -10.0, qymcad_core::feature::Purpose::Real);
    let v = p.add_line_entity(si, -10.0, 0.0, -10.0, len, qymcad_core::feature::Purpose::Real);
    p.add_pattern(si, &[h], PatternKind::Linear { dx: 0.0, dy: 20.0, count: n, dx2: 0.0, dy2: 0.0, count2: 0 });
    p.add_pattern(si, &[v], PatternKind::Linear { dx: 20.0, dy: 0.0, count: n, dx2: 0.0, dy2: 0.0, count2: 0 });
    p
}

fn bigs() -> Vec<Big> {
    let solved = |mut p: Project| {
        p.solve_sketch(0);
        p
    };
    vec![
        Big { name: "2 000 separate rectangles", project: solved(build(Kind::Rectangles, 2_000)) },
        Big { name: "a grid of two patterns, 60 by 60", project: grid(60) },
        Big { name: "a pattern of 200 rectangles in one part", project: solved(build(Kind::Array, 200)) },
        Big { name: "2 000 mixed shapes", project: solved(build(Kind::Mixed, 2_000)) },
    ]
}

/// The lines of the sketch, the first `n`.
fn lines(p: &Project, n: usize) -> Vec<Id> {
    p.sketches[0].entities.iter().filter(|e| matches!(e.kind, EntityKind::Line { .. }) && !e.construction).map(|e| e.id).take(n).collect()
}

/// The corners of the sketch where exactly two lines meet, the first `n`.
fn corners(p: &Project, n: usize) -> Vec<CornerAt> {
    let mut out = Vec::new();
    for q in &p.sketches[0].points {
        if let [pair] = p.vertex_pairs(0, q.id)[..] {
            out.push(CornerAt { point: q.id, pair });
            if out.len() == n {
                break;
            }
        }
    }
    out
}

/// A point inside the first line, a third along it.
fn on_first_line(p: &Project) -> Option<(Id, Point2)> {
    let s = &p.sketches[0];
    s.entities.iter().find_map(|e| match e.kind {
        EntityKind::Line { a, b } if !e.construction => {
            let (pa, pb) = (s.points.iter().find(|q| q.id == a)?, s.points.iter().find(|q| q.id == b)?);
            Some((e.id, Point2::new(pa.x + (pb.x - pa.x) / 3.0, pa.y + (pb.y - pa.y) / 3.0)))
        }
        _ => None,
    })
}

/// A tool, by its name, doing its work on the first sketch of a project.
struct Tool {
    name: &'static str,
    work: fn(&mut Project),
}

fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "a linear pattern of 10",
            work: |p| {
                let _ = p.add_pattern(0, &lines(p, 1), PatternKind::Linear { dx: 3.0, dy: 3.0, count: 10, dx2: 0.0, dy2: 0.0, count2: 0 });
            },
        },
        Tool {
            name: "a pattern changed to 20",
            work: |p| {
                let _ = p.add_pattern(0, &lines(p, 1), PatternKind::Linear { dx: 3.0, dy: 3.0, count: 10, dx2: 0.0, dy2: 0.0, count2: 0 });
                let pi = p.sketches[0].patterns.len() - 1;
                p.update_pattern(0, pi, PatternKind::Linear { dx: 3.0, dy: 3.0, count: 20, dx2: 0.0, dy2: 0.0, count2: 0 });
            },
        },
        Tool {
            name: "a circular pattern of 8",
            work: |p| {
                let _ = p.add_pattern(0, &lines(p, 1), PatternKind::Circular { cx: 0.0, cy: 0.0, count: 8, total_deg: 360.0 });
            },
        },
        Tool { name: "a linear array of 10", work: |p| p.array_linear(0, &lines(p, 1), 3.0, 3.0, 10) },
        Tool { name: "a circular array of 8", work: |p| p.array_circular(0, &lines(p, 1), 0.0, 0.0, 8, 360.0) },
        Tool { name: "50 lines mirrored", work: |p| p.mirror_entities(0, &lines(p, 50), 0.0, -100.0, 1.0, -100.0) },
        Tool {
            name: "50 lines copied",
            work: |p| {
                let _ = p.copy_entities(0, &lines(p, 50), 0.0, -500.0);
            },
        },
        Tool {
            name: "50 lines pasted",
            work: |p| {
                let clip = p.copy_sketch_geometry(0, &lines(p, 50), 0.0, 0.0);
                let _ = p.paste_sketch_geometry(0, &clip, 0.0, -500.0);
            },
        },
        Tool { name: "50 lines moved", work: |p| p.move_entities(0, &lines(p, 50), 1.0, 1.0) },
        Tool { name: "50 lines turned", work: |p| p.rotate_entities(0, &lines(p, 50), 0.0, 0.0, 5.0) },
        Tool { name: "50 lines scaled", work: |p| p.scale_entities(0, &lines(p, 50), 0.0, 0.0, 1.1) },
        Tool {
            name: "20 lines offset",
            work: |p| {
                let _ = p.offset_entities(0, &lines(p, 20), 1.0);
            },
        },
        Tool {
            name: "20 lines made construction",
            work: |p| {
                let _ = p.toggle_construction(0, &lines(p, 20));
            },
        },
        Tool { name: "50 lines deleted", work: |p| p.delete_entities(0, &lines(p, 50)) },
        Tool {
            name: "20 corners filleted as a set",
            work: |p| {
                let _ = p.cut_corner_set(0, &corners(p, 20), CornerCut::Fillet(FilletSize::radius(0.5)), None);
            },
        },
        Tool {
            name: "20 corners chamfered as a set",
            work: |p| {
                let _ = p.cut_corner_set(0, &corners(p, 20), CornerCut::Chamfer(ChamferLegs::equal(0.5)), None);
            },
        },
        Tool {
            name: "every corner of 50 lines filleted",
            work: |p| {
                let only: std::collections::HashSet<Id> = lines(p, 50).into_iter().collect();
                let _ = p.fillet_all_corners_by(0, FilletSize::radius(0.3), Some(&only));
            },
        },
        Tool {
            name: "a line trimmed",
            work: |p| {
                if let Some((e, at)) = on_first_line(p) {
                    let _ = p.trim_line(0, e, at.x, at.y);
                }
            },
        },
        Tool {
            name: "a line broken",
            work: |p| {
                if let Some((e, at)) = on_first_line(p) {
                    let _ = p.break_line(0, e, at.x, at.y);
                }
            },
        },
        Tool {
            name: "a line extended",
            work: |p| {
                if let Some((e, at)) = on_first_line(p) {
                    let _ = p.extend_line(0, e, at.x, at.y);
                }
            },
        },
        Tool {
            name: "close points stitched",
            work: |p| {
                let _ = p.merge_close_points(0, 0.01);
            },
        },
        Tool {
            name: "a line drawn",
            work: |p| {
                let _ = p.add_line_entity(0, -50.0, -50.0, -40.0, -45.0, qymcad_core::feature::Purpose::Real);
            },
        },
        Tool {
            name: "a rectangle drawn",
            work: |p| {
                let _ = p.add_rect_entity(0, -80.0, -80.0, -60.0, -70.0, qymcad_core::feature::Purpose::Real);
            },
        },
        Tool {
            name: "a circle drawn",
            work: |p| {
                let _ = p.add_circle_entity(0, -100.0, -100.0, 5.0, qymcad_core::feature::Purpose::Real);
            },
        },
        Tool {
            name: "a polygon drawn",
            work: |p| {
                let _ = p.add_polygon_entity(0, Point2::new(-120.0, -120.0), Point2::new(-115.0, -120.0), 6, qymcad_core::feature::Purpose::Real);
            },
        },
        Tool { name: "a slot drawn", work: |p| p.add_slot_entity(0, Point2::new(-150.0, -150.0), Point2::new(-140.0, -150.0), 2.0, qymcad_core::feature::Purpose::Real) },
        Tool {
            name: "a constraint laid and solved",
            work: |p| {
                let s = &p.sketches[0];
                let (a, b) = (s.points[s.points.len() - 1].id, s.points[s.points.len() - 2].id);
                let _ = p.add_constraint_if_independent(0, Constraint::Distance { a, b, d: 7.0, off: 2.0, expr: String::new(), driven: false, axis: 0, at: None });
                let _ = p.solve_sketch(0);
            },
        },
    ]
}

#[test]
fn every_tool_takes_a_few_rebuilds_of_a_big_sketch() {
    let mut failures = Vec::new();
    for big in bigs() {
        // one rebuild of this sketch, the best of three: the unit the tools are told against
        let rebuild = (0..3)
            .map(|_| {
                let mut p = big.project.clone();
                let started = Instant::now();
                p.regen_sketch(0);
                started.elapsed()
            })
            .min()
            .unwrap_or_default();
        let budget = rebuild * 5 + Duration::from_millis(200);
        for tool in tools() {
            let mut p = big.project.clone();
            let started = Instant::now();
            (tool.work)(&mut p);
            let t = started.elapsed();
            eprintln!("{:<45} {:<32} {t:>12.3?}  (a rebuild {rebuild:.3?})", big.name, tool.name);
            if t > budget {
                failures.push(format!(
                    "{} - {}: {t:.3?}, {:.0} rebuilds of the sketch ({rebuild:.3?} each), budget {budget:.3?}",
                    big.name,
                    tool.name,
                    t.as_secs_f64() / rebuild.as_secs_f64().max(1e-9)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "tools that take more than a few rebuilds of a big sketch:\n{}", failures.join("\n"));
}
