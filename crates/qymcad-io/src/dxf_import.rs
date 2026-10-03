//! Importing DXF into exact primitive curves — segments, arcs and circles — rather than a tessellation.
//!
//! Read: LINE, CIRCLE, ARC and polylines, including bulge arcs from fillets; ELLIPSE and SPLINE as runs of segments
//! whose corners lie on the curve, fine enough that no chord strays from it by more than `CHORD` mm; INSERT as the
//! entities of its block, placed, turned and scaled, repeated over its rows and columns. A circle stays a circle and an
//! arc stays an arc; they are not broken into segments, or importing a drawing would yield thousands of them. Every
//! other entity is counted by its kind and named, not silently dropped: a drawing that lost its texts or hatches must
//! say so. The sketch recovers connectivity and closure from shared endpoints, so nothing needs stitching here.

use dxf::entities::{Entity, EntityType};
use dxf::Drawing;
use qymcad_core::geom::{Point2, ProfEdge};

use crate::ImportedSketch;

/// How far a chord of an ellipse or a spline may stray from the curve, in mm.
const CHORD: f64 = 0.01;

/// How deep blocks may be inserted into blocks before the rest is taken for a loop.
const DEEPEST: usize = 16;

/// Import a DXF file into a set of exact curves, and the kinds of entity that were not read with how many of each.
pub fn import_dxf(path: &str) -> Result<ImportedSketch, String> {
    let drawing = match Drawing::load_file(path) {
        Ok(d) => d,
        // A SECTION THE IMPORT DOES NOT USE IS NOT A REASON TO REFUSE THE DRAWING. The reader stops at the first pair it
        // does not expect, and a modern CAD writes objects it does not know: both drawings converted from the owner's DWG
        // files were refused over a VISUALSTYLE in OBJECTS. The drawing is read again with only its header, tables,
        // blocks and entities.
        Err(first) => {
            let bytes = std::fs::read(path).map_err(|e| format!("DXF load: {e}"))?;
            let kept = only_needed_sections(&bytes).ok_or_else(|| format!("DXF load: {first}"))?;
            Drawing::load(&mut std::io::Cursor::new(kept)).map_err(|_| format!("DXF load: {first}"))?
        }
    };
    let mut out = ImportedSketch::default();
    let mut skipped: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let entities: Vec<&Entity> = drawing.entities().collect();
    read(&drawing, &entities, &Place::IDENTITY, 0, &mut out.curves, &mut skipped);
    out.skipped = skipped.into_iter().collect();
    Ok(out)
}

/// An ASCII drawing with only the sections the import reads - HEADER, TABLES, BLOCKS and ENTITIES - kept, pair by pair
/// as the file has them; `None` for a binary drawing or one whose pairs do not run in twos.
fn only_needed_sections(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.starts_with(b"AutoCAD Binary DXF") {
        return None;
    }
    let lines: Vec<&[u8]> = bytes.split(|&b| b == b'\n').collect();
    let trim = |l: &[u8]| -> Vec<u8> { l.iter().copied().filter(|&b| b != b'\r').collect::<Vec<u8>>().trim_ascii().to_vec() };
    let mut out = Vec::with_capacity(bytes.len());
    let (mut k, mut keep) = (0, true);
    while k + 1 < lines.len() {
        let (code, value) = (trim(lines[k]), trim(lines[k + 1]));
        if code == b"0" && value == b"SECTION" {
            let name = lines.get(k + 3).map(|l| trim(l)).unwrap_or_default();
            keep = matches!(name.as_slice(), b"HEADER" | b"TABLES" | b"BLOCKS" | b"ENTITIES");
        }
        if keep || (code == b"0" && value == b"EOF") {
            out.extend_from_slice(lines[k]);
            out.push(b'\n');
            out.extend_from_slice(lines[k + 1]);
            out.push(b'\n');
        }
        if code == b"0" && value == b"ENDSEC" {
            keep = true; // between sections, as the file has it
        }
        k += 2;
    }
    Some(out)
}

/// Where the entities of a block stand: x' = a x + b y + tx, y' = c x + d y + ty.
#[derive(Clone, Copy)]
struct Place {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

impl Place {
    const IDENTITY: Place = Place { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };

    fn at(&self, x: f64, y: f64) -> Point2 {
        Point2::new(self.a * x + self.b * y + self.tx, self.c * x + self.d * y + self.ty)
    }

    /// This place after `inner`: a point is placed by `inner` first.
    fn then(&self, inner: &Place) -> Place {
        Place {
            a: self.a * inner.a + self.b * inner.c,
            b: self.a * inner.b + self.b * inner.d,
            c: self.c * inner.a + self.d * inner.c,
            d: self.c * inner.b + self.d * inner.d,
            tx: self.a * inner.tx + self.b * inner.ty + self.tx,
            ty: self.c * inner.tx + self.d * inner.ty + self.ty,
        }
    }

    /// The scale when this place keeps circles circles - the same stretch both ways, square axes - and whether it mirrors.
    fn uniform(&self) -> Option<(f64, bool)> {
        let (sx, sy) = ((self.a * self.a + self.c * self.c).sqrt(), (self.b * self.b + self.d * self.d).sqrt());
        let square = (self.a * self.b + self.c * self.d).abs() <= 1e-12 * sx * sy;
        ((sx - sy).abs() <= 1e-12 * sx.max(sy) && square).then_some((sx, self.a * self.d - self.b * self.c < 0.0))
    }
}

/// Read `entities` placed by `place` into `curves`, counting the kinds not read into `skipped`.
fn read(drawing: &Drawing, entities: &[&Entity], place: &Place, depth: usize, curves: &mut Vec<ProfEdge>, skipped: &mut std::collections::BTreeMap<String, usize>) {
    for entity in entities {
        match &entity.specific {
            EntityType::Line(line) => {
                curves.push(ProfEdge::Line { a: place.at(line.p1.x, line.p1.y), b: place.at(line.p2.x, line.p2.y) });
            }
            EntityType::Circle(c) => match place.uniform() {
                Some((k, _)) => curves.push(ProfEdge::Circle { center: place.at(c.center.x, c.center.y), r: c.radius * k }),
                None => run(|t| place.at(c.center.x + c.radius * t.cos(), c.center.y + c.radius * t.sin()), 0.0, std::f64::consts::TAU, curves),
            },
            EntityType::Arc(a) => {
                let (from, to) = (a.start_angle.to_radians(), a.end_angle.to_radians());
                match place.uniform() {
                    Some((_, mirrored)) => {
                        let arc = arc_from_angles(Point2::new(a.center.x, a.center.y), a.radius, from, to);
                        let ProfEdge::Arc { a: p, b: q, center, .. } = arc else { continue };
                        let (p, q, center) = (place.at(p.x, p.y), place.at(q.x, q.y), place.at(center.x, center.y));
                        curves.push(ProfEdge::Arc { a: p, b: q, center, ccw: !mirrored });
                    }
                    _ => {
                        let to = if to <= from { to + std::f64::consts::TAU } else { to };
                        run(|t| place.at(a.center.x + a.radius * t.cos(), a.center.y + a.radius * t.sin()), from, to, curves);
                    }
                }
            }
            EntityType::LwPolyline(p) => {
                let verts: Vec<(Point2, f64)> = p.vertices.iter().map(|v| (Point2::new(v.x, v.y), v.bulge)).collect();
                placed_polyline(&verts, is_closed_flag(p.flags), place, curves);
            }
            EntityType::Polyline(p) => {
                let verts: Vec<(Point2, f64)> = p.vertices().map(|v| (Point2::new(v.location.x, v.location.y), v.bulge)).collect();
                placed_polyline(&verts, p.is_closed(), place, curves);
            }
            EntityType::Ellipse(e) => {
                // the minor axis is the normal turned across the major one, shortened by the ratio
                let (mx, my) = (e.major_axis.x, e.major_axis.y);
                let turn = if e.normal.z < 0.0 { -1.0 } else { 1.0 };
                let (nx, ny) = (-my * turn * e.minor_axis_ratio, mx * turn * e.minor_axis_ratio);
                let (from, to) = (e.start_parameter, e.end_parameter);
                let to = if to <= from { to + std::f64::consts::TAU } else { to };
                run(|t| place.at(e.center.x + mx * t.cos() + nx * t.sin(), e.center.y + my * t.cos() + ny * t.sin()), from, to, curves);
            }
            EntityType::Spline(sp) => spline(sp, place, curves),
            EntityType::Insert(ins) if depth < DEEPEST => {
                let Some(block) = drawing.blocks().find(|b| b.name == ins.name) else {
                    *skipped.entry("INSERT".into()).or_default() += 1;
                    continue;
                };
                let inner: Vec<&Entity> = block.entities.iter().collect();
                let (sin, cos) = ins.rotation.to_radians().sin_cos();
                for row in 0..ins.row_count.max(1) {
                    for col in 0..ins.column_count.max(1) {
                        // the block's base to its origin, scaled, shifted along its rows and columns, turned, and put down
                        let (ox, oy) = (col as f64 * ins.column_spacing, row as f64 * ins.row_spacing);
                        let local = Place {
                            a: ins.x_scale_factor,
                            b: 0.0,
                            c: 0.0,
                            d: ins.y_scale_factor,
                            tx: -block.base_point.x * ins.x_scale_factor + ox,
                            ty: -block.base_point.y * ins.y_scale_factor + oy,
                        };
                        let turned = Place { a: cos, b: -sin, c: sin, d: cos, tx: ins.location.x, ty: ins.location.y };
                        read(drawing, &inner, &place.then(&turned.then(&local)), depth + 1, curves, skipped);
                    }
                }
            }
            other => {
                let name = format!("{other:?}");
                let kind = name.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("?").to_uppercase();
                *skipped.entry(kind).or_default() += 1;
            }
        }
    }
}

/// A curve given by `at` over `[from, to]` as segments, halved wherever a chord's middle strays from the curve by more
/// than `CHORD`; the corners lie on the curve.
fn run(at: impl Fn(f64) -> Point2, from: f64, to: f64, curves: &mut Vec<ProfEdge>) {
    const START: usize = 32;
    let mut ts: Vec<f64> = (0..=START).map(|k| from + (to - from) * k as f64 / START as f64).collect();
    let mut k = 0;
    while k + 1 < ts.len() && ts.len() < 1 << 16 {
        let (t0, t1) = (ts[k], ts[k + 1]);
        let (p, q, m) = (at(t0), at(t1), at(0.5 * (t0 + t1)));
        if off_chord(p, q, m) > CHORD && t1 - t0 > 1e-9 * (to - from).abs() {
            ts.insert(k + 1, 0.5 * (t0 + t1));
        } else {
            k += 1;
        }
    }
    for w in ts.windows(2) {
        let (p, q) = (at(w[0]), at(w[1]));
        if p.dist(q) > 1e-12 {
            curves.push(ProfEdge::Line { a: p, b: q });
        }
    }
}

/// How far `m` lies from the chord from `p` to `q`.
fn off_chord(p: Point2, q: Point2, m: Point2) -> f64 {
    let (dx, dy) = (q.x - p.x, q.y - p.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return p.dist(m);
    }
    ((m.x - p.x) * dy - (m.y - p.y) * dx).abs() / len
}

/// A SPLINE: the rational B-spline of its control points, knots and weights by de Boor's rule; one given only by the
/// points it runs through goes through them as a Catmull-Rom curve, the way a sketch's own spline does.
fn spline(sp: &dxf::entities::Spline, place: &Place, curves: &mut Vec<ProfEdge>) {
    let ctrl: Vec<(f64, f64)> = sp.control_points.iter().map(|p| (p.x, p.y)).collect();
    let p = sp.degree_of_curve.max(1) as usize;
    let knots = &sp.knot_values;
    if ctrl.len() > p && knots.len() == ctrl.len() + p + 1 {
        let w: Vec<f64> = if sp.weight_values.len() == ctrl.len() { sp.weight_values.clone() } else { vec![1.0; ctrl.len()] };
        let (lo, hi) = (knots[p], knots[ctrl.len()]);
        let at = |t: f64| {
            let t = t.clamp(lo, hi);
            // the span holding t, the last one for its end
            let mut span = p;
            while span + 1 < ctrl.len() && knots[span + 1] <= t {
                span += 1;
            }
            let mut d: Vec<(f64, f64, f64)> = (0..=p)
                .map(|j| {
                    let (x, y) = ctrl[span - p + j];
                    let wj = w[span - p + j];
                    (x * wj, y * wj, wj)
                })
                .collect();
            for r in 1..=p {
                for j in (r..=p).rev() {
                    let i = span - p + j;
                    let den = knots[i + p + 1 - r] - knots[i];
                    let a = if den.abs() < 1e-300 { 0.0 } else { (t - knots[i]) / den };
                    d[j] = ((1.0 - a) * d[j - 1].0 + a * d[j].0, (1.0 - a) * d[j - 1].1 + a * d[j].1, (1.0 - a) * d[j - 1].2 + a * d[j].2);
                }
            }
            let (x, y, wt) = d[p];
            place.at(x / wt, y / wt)
        };
        run(at, lo, hi, curves);
    } else if sp.fit_points.len() >= 2 {
        let pts: Vec<(f64, f64)> = sp.fit_points.iter().map(|q| (q.x, q.y)).collect();
        let n = pts.len();
        for k in 0..n - 1 {
            let p0 = pts[k.saturating_sub(1)];
            let (p1, p2) = (pts[k], pts[k + 1]);
            let p3 = pts[(k + 2).min(n - 1)];
            let at = |t: f64| {
                let (t2, t3) = (t * t, t * t * t);
                let f = |a: f64, b: f64, c: f64, d: f64| 0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (3.0 * b - a - 3.0 * c + d) * t3);
                place.at(f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1))
            };
            run(at, 0.0, 1.0, curves);
        }
    }
}

/// A polyline under a place: exact where the place keeps arcs arcs, its arcs as runs of segments otherwise.
fn placed_polyline(verts: &[(Point2, f64)], closed: bool, place: &Place, curves: &mut Vec<ProfEdge>) {
    let mut local = Vec::new();
    polyline_curves(verts, closed, &mut local);
    for c in local {
        match (c, place.uniform()) {
            (ProfEdge::Line { a, b }, _) => curves.push(ProfEdge::Line { a: place.at(a.x, a.y), b: place.at(b.x, b.y) }),
            (ProfEdge::Arc { a, b, center, ccw }, Some((_, mirrored))) => {
                curves.push(ProfEdge::Arc { a: place.at(a.x, a.y), b: place.at(b.x, b.y), center: place.at(center.x, center.y), ccw: ccw != mirrored })
            }
            (ProfEdge::Arc { a, b, center, ccw }, None) => {
                let r = center.dist(a);
                let (from, mut to) = ((a.y - center.y).atan2(a.x - center.x), (b.y - center.y).atan2(b.x - center.x));
                if ccw && to <= from {
                    to += std::f64::consts::TAU;
                }
                if !ccw && to >= from {
                    to -= std::f64::consts::TAU;
                }
                run(|t| place.at(center.x + r * t.cos(), center.y + r * t.sin()), from, to, curves);
            }
            (ProfEdge::Circle { center, r }, Some((k, _))) => curves.push(ProfEdge::Circle { center: place.at(center.x, center.y), r: r * k }),
            (ProfEdge::Circle { center, r }, None) => run(|t| place.at(center.x + r * t.cos(), center.y + r * t.sin()), 0.0, std::f64::consts::TAU, curves),
        }
    }
}

fn is_closed_flag(flags: i32) -> bool {
    flags & 1 != 0
}

/// An arc from a centre, a radius and two angles; DXF runs counter-clockwise from start to end. The endpoints
/// lie on the circle.
fn arc_from_angles(center: Point2, r: f64, start: f64, end: f64) -> ProfEdge {
    let a = Point2::new(center.x + r * start.cos(), center.y + r * start.sin());
    let b = Point2::new(center.x + r * end.cos(), center.y + r * end.sin());
    ProfEdge::Arc { a, b, center, ccw: true }
}

/// A polyline becomes segments and arcs between adjacent vertices, a non-zero bulge giving an arc. `closed`
/// adds the closing edge from the last vertex to the first.
fn polyline_curves(verts: &[(Point2, f64)], closed: bool, out: &mut Vec<ProfEdge>) {
    let n = verts.len();
    if n < 2 {
        return;
    }
    let last = if closed { n } else { n - 1 };
    for i in 0..last {
        let (p1, bulge) = verts[i];
        let p2 = verts[(i + 1) % n].0;
        if p1.dist(p2) < 1e-9 {
            continue; // a degenerate edge
        }
        out.push(if bulge.abs() > 1e-9 { bulge_arc(p1, p2, bulge) } else { ProfEdge::Line { a: p1, b: p2 } });
    }
}

/// A DXF bulge arc, where `bulge = tan(θ/4)` with θ the inscribed angle and the sign giving the direction. The
/// centre comes from the cotangent formula, and a positive bulge means counter-clockwise.
fn bulge_arc(p1: Point2, p2: Point2, bulge: f64) -> ProfEdge {
    let cot = (1.0 / bulge - bulge) / 2.0;
    let cx = (p1.x + p2.x) / 2.0 - cot * (p2.y - p1.y) / 2.0;
    let cy = (p1.y + p2.y) / 2.0 + cot * (p2.x - p1.x) / 2.0;
    ProfEdge::Arc { a: p1, b: p2, center: Point2::new(cx, cy), ccw: bulge > 0.0 }
}
