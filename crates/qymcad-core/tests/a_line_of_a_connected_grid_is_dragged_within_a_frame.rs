//! A LINE OF A CONNECTED GRID IS DRAGGED WITHIN A FRAME: a horizontal line and a vertical one, each patterned 200 times
//! 20 mm apart, make one grid of 40 000 cells where every line crosses every line across it. A frame of a drag of one
//! line makes again the cells round that line, not the 40 000, and the loops after every frame are those of a whole
//! rebuild.
//!
//! Reported behaviour (#95): in such a grid every drag and every edit waited. The loops of what moved were made again
//! with every curve whose box met one of theirs, and so on - in a connected grid that is every curve: 0.7 s a frame
//! in a release build.
use qymcad_core::feature::Purpose;
use qymcad_core::model::{EntityKind, PatternKind, Project};
use std::time::{Duration, Instant};

/// The grid of the report, `n` lines each way.
fn grid(n: u32) -> Project {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let len = 20.0 * n as f64;
    let h = p.add_line_entity(si, 0.0, -10.0, len, -10.0, Purpose::Real);
    let v = p.add_line_entity(si, -10.0, 0.0, -10.0, len, Purpose::Real);
    p.add_pattern(si, &[h], PatternKind::Linear { dx: 0.0, dy: 20.0, count: n, dx2: 0.0, dy2: 0.0, count2: 0 });
    p.add_pattern(si, &[v], PatternKind::Linear { dx: 20.0, dy: 0.0, count: n, dx2: 0.0, dy2: 0.0, count2: 0 });
    p.solve_sketch(si);
    p
}

/// The loops of a sketch as a sorted list of what each is: closed, its entities, its centroid and area to 1e-6.
fn loops(p: &Project) -> Vec<String> {
    let mut out: Vec<String> = p.sketches[0]
        .contour_ids
        .iter()
        .filter_map(|cid| {
            let c = &p.contours[p.contour_index(*cid)?];
            let mut src = c.edge_src.clone();
            src.sort_unstable();
            src.dedup();
            let n = c.points.len().max(1) as f64;
            let (sx, sy) = c.points.iter().fold((0.0, 0.0), |(a, b), q| (a + q.x, b + q.y));
            Some(format!("{} {:?} ({:.6}, {:.6}) {:.6}", c.closed, src, sx / n, sy / n, c.signed_area().abs()))
        })
        .collect();
    out.sort();
    out
}

/// The second end of a line of the grid: the line `k` of its kind (horizontal or vertical), counted up or right.
fn line_end(p: &Project, horizontal: bool, k: usize) -> (u64, f64, f64) {
    let s = &p.sketches[0];
    let at = |id: u64| s.points.iter().find(|q| q.id == id).map(|q| (q.x, q.y)).unwrap_or_default();
    let mut ends: Vec<(u64, f64, f64)> = s
        .entities
        .iter()
        .filter_map(|e| match e.kind {
            EntityKind::Line { a, b } => {
                let (pa, pb) = (at(a), at(b));
                ((pa.1 == pb.1) == horizontal).then_some((b, pb.0, pb.1))
            }
            _ => None,
        })
        .collect();
    ends.sort_by(|a, b| if horizontal { a.2.total_cmp(&b.2) } else { a.1.total_cmp(&b.1) });
    ends[k]
}

#[test]
fn a_line_of_a_grid_is_dragged_and_its_loops_are_those_of_a_rebuild() {
    // the loops checked against a rebuild on a grid small enough to rebuild after every frame; every case: a line in
    // the middle, a line at the edge, a vertical one, the end led across the next line and back
    let mut failures = Vec::new();
    for (what, horizontal, k) in [("a middle row", true, 5), ("the first row", true, 0), ("the last row", true, 11), ("a middle column", false, 6)] {
        let mut p = grid(12);
        let (id, x, y) = line_end(&p, horizontal, k);
        let (dx, dy) = if horizontal { (0.0, 1.0) } else { (1.0, 0.0) };
        for (f, step) in [3.0, 9.0, 25.0, 9.0, -30.0, 0.0].into_iter().enumerate() {
            let _ = p.solve_sketch_drag_fast(0, Some((id, x + dx * step, y + dy * step)));
            let mut whole = p.clone();
            whole.regen_sketch(0);
            let (framed, rebuilt) = (loops(&p), loops(&whole));
            if framed != rebuilt {
                failures.push(format!("{what}, frame {f}: {} loops against {} of a rebuild; first apart: {:?}", framed.len(), rebuilt.len(), framed.iter().zip(&rebuilt).find(|(a, b)| a != b)));
            }
        }
    }
    assert!(failures.is_empty(), "the loops of a drag frame and of a rebuild differ:\n{}", failures.join("\n"));
}

#[test]
fn a_frame_of_a_drag_in_a_grid_of_200_takes_a_frame() {
    let mut p = grid(200);
    let (id, x, y) = line_end(&p, true, 100);
    let mut frames = Vec::new();
    for step in [1.0, 2.0, 3.0, 4.0, 5.0] {
        let started = Instant::now();
        let _ = p.solve_sketch_drag_fast(0, Some((id, x, y + step)));
        frames.push(started.elapsed());
    }
    eprintln!("a line of a grid of 200 by 200 dragged: frames {frames:?}");
    // measured: 0.7 s a frame while every curve of the grid was taken with the line, 31 ms with the cells round it alone
    // and 14 ms with the loops of the drag on a grid of their own size
    let budget = Duration::from_millis(50);
    let after_first = &frames[1..];
    assert!(after_first.iter().all(|t| *t < budget), "frames of a drag in a grid of 200 by 200 took {frames:?}, budget {budget:?} after the first");
}
