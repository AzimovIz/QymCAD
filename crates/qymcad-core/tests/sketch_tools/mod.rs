//! THE TOOLS OF A SKETCH, each through the door of the project the window calls, on the first sketch of a project: the
//! list the matrices of the time of the tools run over.
#![allow(dead_code)] // each probe that takes this module in uses a part of it

use qymcad_core::geom::Point2;
use qymcad_core::model::{ChamferLegs, Constraint, CornerAt, CornerCut, EntityKind, FilletSize, Id, PatternKind, Project};

/// The lines of the sketch, the first `n`.
pub fn lines(p: &Project, n: usize) -> Vec<Id> {
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
pub struct Tool {
    pub name: &'static str,
    pub work: fn(&mut Project),
}

pub fn tools() -> Vec<Tool> {
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
        Tool {
            name: "a line drawn from a point on a line",
            work: |p| {
                // as the line tool lays it: the line, its first end held on the line under it, the sketch solved
                if let Some((e, at)) = on_first_line(p) {
                    let new = p.add_line_entity(0, at.x, at.y, at.x - 7.0, at.y - 5.0, qymcad_core::feature::Purpose::Real);
                    let ends = p.sketches[0].entities.iter().find(|x| x.id == new).map(|x| x.kind);
                    let line = p.sketches[0].entities.iter().find(|x| x.id == e).map(|x| x.kind);
                    if let (Some(EntityKind::Line { a: start, .. }), Some(EntityKind::Line { a, b })) = (ends, line) {
                        let _ = p.add_constraint_if_independent(0, Constraint::PointOnLine { p: start, a, b });
                    }
                    let _ = p.solve_sketch(0);
                }
            },
        },
        Tool {
            name: "a dimension laid as the window lays it",
            work: |p| {
                // solved, asked whether it is redundant and whether it conflicts (`finish_dim`)
                if let Some(EntityKind::Line { a, b }) = p.sketches[0].entities.iter().find(|e| matches!(e.kind, EntityKind::Line { .. }) && !e.construction).map(|e| e.kind) {
                    let ci = p.sketches[0].constraints.len();
                    let d = p.sketches[0].point(a).zip(p.sketches[0].point(b)).map_or(10.0, |(q, r)| (q.x - r.x).hypot(q.y - r.y));
                    p.sketches[0].constraints.push(Constraint::Distance { a, b, d, off: 2.0, expr: String::new(), driven: false, axis: 0, at: None });
                    p.solve_sketch(0);
                    let _ = p.dim_redundant(0, ci);
                    let _ = p.sketch_conflicts(0);
                }
            },
        },
    ]
}
