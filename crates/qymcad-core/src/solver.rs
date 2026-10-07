//! Geometric constraint solver for a sketch (2D).
//!
//! The variables are point coordinates *and* circle radii: a radius is a real scalar unknown of the solver
//! rather than a field of an entity. Every constraint contributes a residual, and the sum of squares is
//! minimised by Levenberg-Marquardt over an analytic Jacobian.

use std::collections::HashMap;

use crate::model::{Constraint, Id, SketchPoint};

/// Radius unknown of a circle, keyed by the id of its centre (every circle owns its centre).
///
/// Treating the radius as a solver variable on par with coordinates is what makes the degree-of-freedom count,
/// tangency and equal-radius constraints come out right.
#[derive(Clone, Copy, Debug)]
pub struct RadiusVar {
    pub center: Id,
    pub value: f64,
}

/// Solve the constraints over points alone, without radius unknowns — for tests on pure point sets.
pub fn solve(points: &mut [SketchPoint], constraints: &[Constraint]) -> f64 {
    solve_full(points, &mut Vec::new(), constraints, None)
}

/// Solve with a dragged point pulled towards the cursor, without radius unknowns.
pub fn solve_drag(points: &mut [SketchPoint], constraints: &[Constraint], drag: Option<(Id, f64, f64)>) -> f64 {
    solve_full(points, &mut Vec::new(), constraints, drag)
}

/// The full solver: points plus circle radii as unknowns. `drag = Some((id, x, y))` pulls the dragged point
/// towards the cursor; fully constrained geometry resists that pull.
pub fn solve_full(points: &mut [SketchPoint], radii: &mut [RadiusVar], constraints: &[Constraint], drag: Option<(Id, f64, f64)>) -> f64 {
    solve_full_iter(points, radii, constraints, drag, 120)
}

/// Same as `solve_full`, but with an explicit Levenberg-Marquardt iteration budget. An interactive drag gets a
/// smaller budget: responsiveness matters more than accuracy there, the final exact solve happens on release,
/// and every frame is a warm start from the previous coordinates, so iterations accumulate anyway.
///
/// On top of LM sits a side flip for axis dimensions. The residual |Δ| − d has more than a kink at zero — it has
/// a barrier: for a horizontal or vertical dimension to change sides, Δ must pass through 0, which means climbing
/// over a local maximum of the error. A local method never crosses that, and instead of a solvable system it
/// returns a least-squares compromise in which every dimension is off and the nodes go red. The cure is a
/// multi-start: a violated axis dimension gets its point mirrored to the other side and the system is solved
/// again, and the result is accepted only if the residual strictly dropped. Disabled during a drag, where frame
/// stability and responsiveness outweigh it.
pub fn solve_full_iter(points: &mut [SketchPoint], radii: &mut [RadiusVar], constraints: &[Constraint], drag: Option<(Id, f64, f64)>, max_iter: usize) -> f64 {
    let scale = Scale::of(points);
    guarded(points, radii, |points, radii| solve_by_parts(points, radii, constraints, drag, max_iter, Solve { scale, algebra: Algebra::BySize }))
}

/// The solve of `solve_full_iter` with every part solved by the sparse algebra, whatever its size - the check of that
/// algebra against the dense one on parts small enough for both.
pub fn solve_full_iter_sparse(points: &mut [SketchPoint], radii: &mut [RadiusVar], constraints: &[Constraint], drag: Option<(Id, f64, f64)>, max_iter: usize) -> f64 {
    let scale = Scale::of(points);
    guarded(points, radii, |points, radii| solve_by_parts(points, radii, constraints, drag, max_iter, Solve { scale, algebra: Algebra::Sparse }))
}

/// The whole sketch as one system, the way it was solved before it was split into parts (`solve_full_iter`). Kept as
/// the reference the parts are checked against: on a sketch whose parts are solved alone, the answer is the same.
pub fn solve_full_iter_whole(points: &mut [SketchPoint], radii: &mut [RadiusVar], constraints: &[Constraint], drag: Option<(Id, f64, f64)>, max_iter: usize) -> f64 {
    let scale = Scale::of(points);
    guarded(points, radii, |points, radii| solve_full_iter_inner(points, radii, constraints, drag, max_iter, Solve { scale, algebra: Algebra::Dense }))
}

/// Run a solve, and put the sketch back as it was if it comes out with a non-number.
fn guarded(points: &mut [SketchPoint], radii: &mut [RadiusVar], solve: impl FnOnce(&mut [SketchPoint], &mut [RadiusVar]) -> f64) -> f64 {
    // The solver must never be able to corrupt the sketch.
    //
    // A numerical method can produce non-numbers: a degenerate system, huge magnitudes, a division by
    // near-zero inside the Jacobian. Once a NaN reaches point coordinates it never leaves — it is saved into
    // the document, spreads into bodies, bounding boxes and the view, and the sketch is broken for good without
    // a single message. So the input is remembered and the output is checked: if the result is not a number,
    // the previous state is restored and an infinite residual is reported, leaving the sketch red but the
    // geometry intact. Returning garbage silently is not an option.
    let backup: Vec<SketchPoint> = points.to_vec();
    let backup_r: Vec<RadiusVar> = radii.to_vec();
    let res = solve(points, radii);
    let clean = res.is_finite() && points.iter().all(|p| p.x.is_finite() && p.y.is_finite()) && radii.iter().all(|r| r.value.is_finite());
    if clean {
        return res;
    }
    points.clone_from_slice(&backup);
    radii.clone_from_slice(&backup_r);
    f64::INFINITY
}

/// The size of the whole sketch, which the thresholds of a solve scale with. Taken from the whole sketch even when
/// one part is solved, so a part is solved to the same thresholds as it was inside the whole.
#[derive(Clone, Copy)]
struct Scale {
    /// the largest coordinate, at least 1: the residual below `1e-4 * span` counts as solved
    span: f64,
    /// the spread of all coordinates: an axis dimension off by more than `1e-4` of it is a candidate for a side flip
    extent: f64,
}

impl Scale {
    fn of(points: &[SketchPoint]) -> Scale {
        let span = points.iter().fold(0.0_f64, |m, p| m.max(p.x.abs()).max(p.y.abs())).max(1.0);
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for p in points {
            lo = lo.min(p.x).min(p.y);
            hi = hi.max(p.x).max(p.y);
        }
        Scale { span, extent: if hi > lo { hi - lo } else { 0.0 } }
    }
}

/// How the system of a step is solved.
#[derive(Clone, Copy, PartialEq)]
enum Algebra {
    /// Gauss-Jordan on the dense normal matrix, whatever the size
    Dense,
    /// the sparse Cholesky of the normal matrix, whatever the size
    Sparse,
    /// dense up to `DENSE_UP_TO` free unknowns, sparse past it
    BySize,
}

/// Up to this many free unknowns a step is solved dense. A sketch part of a few shapes - nearly every part there is -
/// is solved as it always was; the dense matrix of 8 000 unknowns (an array of 1 000 rectangles) is 512 MB and
/// its Gauss-Jordan 5e11 operations a step.
const DENSE_UP_TO: usize = 200;

/// What a solve keeps through all of its runs: the size it scales with and its algebra.
#[derive(Clone, Copy)]
struct Solve {
    scale: Scale,
    algebra: Algebra,
}

/// A part of a sketch that no constraint ties to the rest: its points, the radii of its circles and its constraints,
/// by their places in the sketch's lists.
struct Part {
    points: Vec<usize>,
    radii: Vec<usize>,
    constraints: Vec<usize>,
}

/// The points, radii and constraints of one part, copied out of the sketch's lists.
struct Own {
    points: Vec<SketchPoint>,
    radii: Vec<RadiusVar>,
    constraints: Vec<Constraint>,
}

impl Part {
    fn own(&self, points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> Own {
        Own {
            points: self.points.iter().map(|&i| points[i]).collect(),
            radii: self.radii.iter().map(|&j| radii[j]).collect(),
            constraints: self.constraints.iter().map(|&ci| constraints[ci].clone()).collect(),
        }
    }
}

/// The parts of a sketch: the points a constraint names are of one part, and a radius is of the part of its centre.
/// A constraint that names no point of the sketch is of no part - the solve leaves it out anyway (`cons_ok`).
fn parts(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> Vec<Part> {
    let idx: HashMap<Id, usize> = points.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
    let mut up: Vec<usize> = (0..points.len()).collect();
    fn root(up: &mut [usize], mut i: usize) -> usize {
        while up[i] != i {
            up[i] = up[up[i]];
            i = up[i];
        }
        i
    }
    let mut first_of: Vec<Option<usize>> = Vec::with_capacity(constraints.len());
    for c in constraints {
        let mut first = None;
        for id in c.points() {
            let Some(&i) = idx.get(&id) else { continue };
            match first {
                None => first = Some(i),
                Some(f) => {
                    let (ra, rb) = (root(&mut up, f), root(&mut up, i));
                    if ra != rb {
                        up[ra.max(rb)] = ra.min(rb);
                    }
                }
            }
        }
        first_of.push(first);
    }
    let mut part_of_root: HashMap<usize, usize> = HashMap::new();
    let mut out: Vec<Part> = Vec::new();
    let mut part_at = |up: &mut [usize], i: usize, out: &mut Vec<Part>| -> usize {
        let r = root(up, i);
        *part_of_root.entry(r).or_insert_with(|| {
            out.push(Part { points: Vec::new(), radii: Vec::new(), constraints: Vec::new() });
            out.len() - 1
        })
    };
    for i in 0..points.len() {
        let k = part_at(&mut up, i, &mut out);
        out[k].points.push(i);
    }
    for (j, rv) in radii.iter().enumerate() {
        if let Some(&i) = idx.get(&rv.center) {
            let k = part_at(&mut up, i, &mut out);
            out[k].radii.push(j);
        }
    }
    for (ci, first) in first_of.into_iter().enumerate() {
        if let Some(i) = first {
            let k = part_at(&mut up, i, &mut out);
            out[k].constraints.push(ci);
        }
    }
    out
}

/// Solve every part of the sketch as a system of its own. 300 separate lines are 300 systems of 4 unknowns, not one
/// of 1 200: the dense algebra of a solve costs the cube of its unknowns. A part with no constraint is not solved. A
/// drag solves only the part of the dragged point - nothing ties the others to it, and the release solves them all.
fn solve_by_parts(points: &mut [SketchPoint], radii: &mut [RadiusVar], constraints: &[Constraint], drag: Option<(Id, f64, f64)>, max_iter: usize, how: Solve) -> f64 {
    let drag = drag.filter(|(id, _, _)| points.iter().any(|p| p.id == *id));
    // the parts to solve, each with the drag if it holds the dragged point
    let work: Vec<ToSolve> = parts(points, radii, constraints)
        .into_iter()
        .map(|part| {
            let drag = drag.filter(|(id, _, _)| part.points.iter().any(|&i| points[i].id == *id));
            ToSolve { part, drag }
        })
        .filter(|w| !(w.part.constraints.is_empty() && w.drag.is_none() || drag.is_some() && w.drag.is_none()))
        .collect();
    let (shared_points, shared_radii): (&[SketchPoint], &[RadiusVar]) = (points, radii);
    let solved = each_part(&work, |w| {
        let mut own = w.part.own(shared_points, shared_radii, constraints);
        let residual = solve_full_iter_inner(&mut own.points, &mut own.radii, &own.constraints, w.drag, max_iter, how);
        Solved { own, residual }
    });
    // put back and summed in the order of the parts, whatever thread solved which
    let mut sum_sq = 0.0;
    for (w, Solved { own, residual }) in work.iter().zip(solved) {
        sum_sq += residual * residual;
        for (&i, q) in w.part.points.iter().zip(own.points) {
            points[i] = q;
        }
        for (&j, rv) in w.part.radii.iter().zip(own.radii) {
            radii[j] = rv;
        }
    }
    sum_sq.sqrt()
}

/// A part to solve, with the drag if it holds the dragged point.
struct ToSolve {
    part: Part,
    drag: Option<(Id, f64, f64)>,
}

/// A part solved: its points and radii as they came out, and what is left of its constraints.
struct Solved {
    own: Own,
    residual: f64,
}

/// Fewer parts than this are solved on the calling thread: a thread costs more to start than a few parts to solve.
const PARTS_FOR_A_THREAD: usize = 256;

/// `solve` over every item of `work`, the answers in the order of `work`. Spread over `threads()` threads in runs of
/// items one after another, where there are enough items for each thread to take `PARTS_FOR_A_THREAD`; each item is
/// solved alone, so the answers do not depend on how many threads there are.
fn each_part<W: Sync, R: Send>(work: &[W], solve: impl Fn(&W) -> R + Sync) -> Vec<R> {
    let threads = threads().min(work.len() / PARTS_FOR_A_THREAD).max(1);
    if threads == 1 {
        return work.iter().map(&solve).collect();
    }
    let run = work.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let handles: Vec<_> = work.chunks(run).map(|chunk| scope.spawn(|| chunk.iter().map(&solve).collect::<Vec<R>>())).collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
    })
}

/// HOW MANY THREADS A SOLVE MAY SPREAD ITS PARTS OVER, as said by `set_threads`; zero, the start, is all the cores but
/// one.
static THREADS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// SAY HOW MANY THREADS A SOLVE MAY TAKE: the setting of the program for the cores of a computation, handed over with
/// the kernel's own (`qymcad_kernel::set_parallel`) - one number for all of it. One is a single thread.
pub fn set_threads(n: usize) {
    THREADS.store(n, std::sync::atomic::Ordering::Relaxed);
}

/// How many threads a solve takes now (`set_threads`).
pub fn threads() -> usize {
    match THREADS.load(std::sync::atomic::Ordering::Relaxed) {
        0 => std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).saturating_sub(1).max(1),
        n => n,
    }
}

fn solve_full_iter_inner(points: &mut [SketchPoint], radii: &mut [RadiusVar], constraints: &[Constraint], drag: Option<(Id, f64, f64)>, max_iter: usize, how: Solve) -> f64 {
    let scale = how.scale;
    // Two stages. First, a solve with a pull towards the previous state, which selects the solution closest to
    // how the sketch currently looks — without it the free degrees of freedom, such as the rotation of a
    // polygon, drift anywhere. Second, a polish without that pull, started from the solution just found: the
    // constraints are driven to machine precision, and there is nowhere left to travel in the null space
    // because the start already sits in the right place.
    let mut best = solve_lm(points, radii, constraints, drag, max_iter, Pull::SETTLE, how.algebra);
    // The polish runs only if the system is solvable. For a contradictory sketch the solution is a
    // least-squares compromise, and there is a whole set of such compromises: the polish would wander across
    // it, and re-solving would shift the geometry — the property test for a sketch drifting between solves
    // caught a 27 mm move. When the constraints are satisfiable, driving them to machine precision is exactly
    // what is wanted.
    //
    // The threshold scales with the sketch, it is not absolute: on a 500 mm part the residual left by the first
    // stage exceeds any fixed threshold on its own, the polish never started, and a dimension landed with a
    // 2e-4 error. Same lesson as the thresholds for dimension conflicts.
    let span = scale.span;
    // The arms of angle dimensions are held at their old lengths only to make an arm turn rather than stretch;
    // where the other constraints force an arm to a new length, that hold fights them and the first stage stops
    // at a compromise. A chamfer by a leg and an angle, its angle changed from 30 to 45 deg, must lengthen its
    // cut from 5.77 to 7.07 and stopped at a residual of 0.057. Solved again from there without the hold, and
    // taken only if that solves the sketch, so a contradictory one keeps the compromise it had. Measured over the probes
    // of the core: 240 solves of 48 063 ran it, all on parts of at most 36 unknowns, 0.27 s in all, and 2 were solved
    // by it. It repeats the part it is called for, not the sketch (`solve_by_parts`).
    if drag.is_none() && best >= 1e-4 * span {
        let mut trial: Vec<SketchPoint> = points.to_vec();
        let mut trial_radii: Vec<RadiusVar> = radii.to_vec();
        let r = solve_lm(&mut trial, &mut trial_radii, constraints, drag, max_iter, Pull { hold_arms: false, ..Pull::SETTLE }, how.algebra);
        if r < 1e-4 * span {
            best = r;
            points.copy_from_slice(&trial);
            radii.copy_from_slice(&trial_radii);
        }
    }
    if best < 1e-4 * span {
        best = solve_lm(points, radii, constraints, drag, 40, Pull::POLISH, how.algebra).min(best);
    }
    const SOLVED: f64 = 1e-4; // below this the system counts as solved and there is nothing left to try
    if drag.is_some() || best <= SOLVED {
        return best;
    }
    for (a, b, axis) in violated_axis_dims(points, constraints, scale.extent, 6) {
        let (Some(ia), Some(ib)) = (points.iter().position(|p| p.id == a), points.iter().position(|p| p.id == b)) else { continue };
        let mut trial: Vec<SketchPoint> = points.to_vec();
        let mut trial_radii: Vec<RadiusVar> = radii.to_vec();
        // mirror point `b` about `a` along the axis of the dimension: start from the other side of the barrier
        if axis == 1 {
            trial[ib].x = 2.0 * trial[ia].x - trial[ib].x;
        } else {
            trial[ib].y = 2.0 * trial[ia].y - trial[ib].y;
        }
        let r = solve_lm(&mut trial, &mut trial_radii, constraints, drag, max_iter, Pull::SETTLE, how.algebra);
        if r < best - 1e-9 {
            best = r;
            points.copy_from_slice(&trial);
            radii.copy_from_slice(&trial_radii);
            if best <= SOLVED {
                break;
            }
        }
    }
    best
}

/// The pulls of one Levenberg-Marquardt run towards the state it started from: `reg` towards the old coordinates,
/// `hold_arms` - the arms of angle dimensions kept at their old lengths - and the starting damping `lambda0`.
#[derive(Clone, Copy)]
struct Pull {
    reg: f64,
    lambda0: f64,
    hold_arms: bool,
}

impl Pull {
    /// The first stage: the solution nearest to how the sketch looks now.
    const SETTLE: Pull = Pull { reg: 1e-3, lambda0: 1e-3, hold_arms: true };
    /// The polish of a solved sketch to machine precision. It starts on a solution, so there is no turn left to
    /// choose, and an arm held at a length 1e-6 off its solved one kept a residual of 1.9e-6.
    const POLISH: Pull = Pull { reg: 1e-9, lambda0: 0.0, hold_arms: false };
}

/// Axis (horizontal or vertical) dimensions that the current geometry does not satisfy — the candidates for a
/// side flip. The tolerance scales with the sketch, on the same principle as `sketch_conflicts`: the solver
/// repairs exactly what is shown as red, and behaves the same on a 2 mm part and on a 3 m frame. At most
/// `limit` of them.
fn violated_axis_dims(points: &[SketchPoint], constraints: &[Constraint], extent: f64, limit: usize) -> Vec<(Id, Id, u8)> {
    let pos: HashMap<Id, (f64, f64)> = points.iter().map(|p| (p.id, (p.x, p.y))).collect();
    // Tolerance scaled by the extent of the sketch (`Scale::extent`), the same principle as in `sketch_conflicts`: a
    // fixed 0.05 mm kept the multi-start from firing on a metre-sized frame, and on a tiny part it would have
    // declared almost everything violated.
    let tol = (extent * 1e-4).max(1e-6);
    let mut out = Vec::new();
    for c in constraints {
        if let Constraint::Distance { a, b, d, axis, driven, .. } = c {
            if *driven || (*axis != 1 && *axis != 2) {
                continue;
            }
            let (Some(&(ax, ay)), Some(&(bx, by))) = (pos.get(a), pos.get(b)) else { continue };
            let m = if *axis == 1 { (ax - bx).abs() } else { (ay - by).abs() };
            if (m - *d).abs() > tol {
                out.push((*a, *b, *axis));
                if out.len() >= limit {
                    break;
                }
            }
        }
    }
    out
}

/// A single Levenberg-Marquardt run, without the multi-start side flip.
// THE INDEX IS THE MEANING: `a[i][i]` is the DIAGONAL of the normal matrix. An iterator over rows would
// still have to index the column, and the damping would stop being visibly a diagonal one.
#[allow(clippy::needless_range_loop)]
fn solve_lm(points: &mut [SketchPoint], radii: &mut [RadiusVar], constraints: &[Constraint], drag: Option<(Id, f64, f64)>, max_iter: usize, pull: Pull, algebra: Algebra) -> f64 {
    let Pull { reg: w_reg, lambda0, hold_arms } = pull;
    if points.is_empty() {
        return 0.0;
    }
    let np = points.len();
    let idx: HashMap<Id, usize> = points.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
    // radius unknowns go into x after the point coordinates: x[np * 2 + j]
    let ridx: HashMap<Id, usize> = radii.iter().enumerate().map(|(j, rv)| (rv.center, np * 2 + j)).collect();

    let has = |id: Id| idx.contains_key(&id);
    let is_center = |id: Id| ridx.contains_key(&id);
    let cons: Vec<Constraint> = constraints.iter().filter(|&c| cons_ok(c, &has, &is_center)).cloned().collect();
    let drag = drag.and_then(|(id, x, y)| idx.get(&id).map(|&i| (i, x, y)));
    if cons.is_empty() && drag.is_none() {
        return 0.0;
    }

    let anchor: HashMap<Id, (f64, f64)> = cons
        .iter()
        .filter_map(|c| match *c {
            Constraint::Fixed { p } => idx.get(&p).map(|&i| (p, (points[i].x, points[i].y))),
            _ => None,
        })
        .collect();

    let nv = np * 2 + radii.len();
    let mut x: Vec<f64> = points.iter().flat_map(|p| [p.x, p.y]).collect();
    x.extend(radii.iter().map(|rv| rv.value));

    let x0 = x.clone();
    // `w_reg` is the weight of the pull towards the previous coordinates and is supplied from the outside, see
    // `solve_full_iter`.
    //
    // It exists as a tie-breaker: out of the set of solutions, pick the one closest to how the sketch looks
    // now. Without it the free degrees of freedom drift — a polygon rotates about its centre — and for a
    // contradictory system the compromise is not unique at all, so the solution moves between calls.
    //
    // It must not, however, compete with the constraints. At weight 1e-3 the solution settled into an
    // equilibrium between a dimension and the pull towards the old position, with an error of roughly
    // 1e-6 · displacement — enough to yield −129.99900 instead of −130. Hence the two stages: weight 1e-3
    // first, which steers towards the nearest solution, then a polish at 1e-9, which keeps the tie-breaker
    // while pushing the distortion of dimensions below machine precision.
    //
    // The mouse weight sits below the weight of the constraints, not above it. At 5.0 the cursor outranked any
    // dimension fivefold, and a fully constrained 40×25 rectangle stretched some fifty millimetres after the
    // mouse before snapping back on release. The dimensions themselves were fine; what lied was the display,
    // and the reading was that the dimensions do not hold. Fully constrained geometry must not move at all
    // under a drag — the reasoning is written up at `solve_sketch_drag`.
    //
    // Why a small weight suffices instead of projecting onto the null space: along the free directions the
    // constraints have no gradient, so nothing competes with the mouse and the point reaches the cursor at any
    // weight. Along the constrained directions the equilibrium distorts a constraint by w²/(1+w²) of the
    // displacement — 96 % at weight 5.0, which is what was observed, and 0.04 % at 0.02, below line thickness
    // for any reasonable drag.
    //
    // The weight is bounded from below twice: by the `w_reg` tie-breaker (1e-3), which the mouse has to
    // outweigh or the point sticks to its old position, and by the conditioning of the system, see the
    // measurements below.
    //
    // The value comes from a measured curve rather than from picking something small. At 0.02 the quality is
    // excellent — a constrained sketch does not budge — but the performance guard showed the drag frame on a
    // large sketch rising to 191 ms from 97: too small a weight conditions the system poorly and LM burns the
    // whole iteration budget. Point by point:
    //
    //     weight   drag frame   drift of a constrained sketch
    //     5.0        101 ms       51.9 mm   <- mouse stronger than the constraints
    //     1.0         81 ms       33.5 mm
    //     0.3         35 ms        7.7 mm
    //     0.1         94 ms        1.0 mm
    //     0.05        55 ms      < 0.5 mm   <- chosen
    //     0.02       191 ms      < 0.5 mm
    //
    // 0.05 takes both: the constraints hold, with drift below line thickness, and the frame costs half of what
    // it used to.
    let w_drag = 0.05_f64;
    // Preserve the arm lengths of angle constraints. The angular residual is length-independent — it only
    // rotates — but a free arm keeps its radial freedom, so the solver projected the endpoint onto the nearest
    // point of the ray and destroyed the length. Holding the arm lengths softly at their pre-solve values, taken
    // from `x0`, makes a rotation about the vertex satisfy both the angle and the length at once (both residuals
    // reach zero), so the arm rotates instead of stretching. The weight is above the positional regularisation,
    // which is what selects rotation, and far below `Distance` constraints, so explicit lengths still win.
    let w_len = 1e-1_f64;
    let angle_arms: Vec<(usize, usize, f64)> = if !hold_arms {
        Vec::new()
    } else {
        // arms whose length is already set by a `Distance` dimension are left alone: the dimension holds them
        // and there is nothing to interfere with
        let dimensioned: std::collections::HashSet<(Id, Id)> = cons
            .iter()
            .filter_map(|c| match *c {
                Constraint::Distance { a, b, .. } => Some(if a < b { (a, b) } else { (b, a) }),
                _ => None,
            })
            .collect();
        let arm = |p: Id, q: Id| -> Option<(usize, usize, f64)> {
            if dimensioned.contains(&if p < q { (p, q) } else { (q, p) }) {
                return None;
            }
            let (ip, iq) = (*idx.get(&p)?, *idx.get(&q)?);
            let d = ((x0[2 * ip] - x0[2 * iq]).powi(2) + (x0[2 * ip + 1] - x0[2 * iq + 1]).powi(2)).sqrt();
            Some((ip, iq, d))
        };
        cons.iter()
            .flat_map(|c| match *c {
                Constraint::Angle { a, b, c: cc, .. } => vec![arm(b, a), arm(b, cc)],
                Constraint::AngleLines { a, b, c: cc, d, .. } => vec![arm(a, b), arm(cc, d)],
                _ => Vec::new(),
            })
            .flatten()
            .collect()
    };
    let residuals = |x: &[f64]| {
        let mut r = residuals_of(x, &x0, &idx, &ridx, &anchor, &cons);
        for k in 0..nv {
            r.push(w_reg * (x[k] - x0[k]));
        }
        for &(ip, iq, l0) in &angle_arms {
            let (dx, dy) = (x[2 * ip] - x[2 * iq], x[2 * ip + 1] - x[2 * iq + 1]);
            r.push(w_len * ((dx * dx + dy * dy).sqrt() - l0));
        }
        if let Some((i, tx, ty)) = drag {
            r.push(w_drag * (x[2 * i] - tx));
            r.push(w_drag * (x[2 * i + 1] - ty));
        }
        r
    };

    // row count of every constraint, used to lay the Jacobian out in the shared matrix
    let con_nrows: Vec<usize> = cons
        .iter()
        .map(|c| {
            let mut t = Vec::new();
            con_rows(c, &x, &x0, &idx, &ridx, &anchor, &mut t);
            t.len()
        })
        .collect();
    // variables of anchored points are not solved for, see the assembly of the system below
    let mut fixed_var = vec![false; nv];
    for c in cons.iter() {
        if let Constraint::Fixed { p } = *c {
            if let Some(&i) = idx.get(&p) {
                fixed_var[2 * i] = true;
                fixed_var[2 * i + 1] = true;
            }
        }
    }
    // ...but the dragged point is always solved for: the gesture outranks the anchor while the mouse holds it
    if let Some((i, _, _)) = drag {
        fixed_var[2 * i] = false; // by this point `drag.0` is the index of the point, not its id: see the call site
        fixed_var[2 * i + 1] = false;
    }
    let mut lambda = lambda0;
    for _ in 0..max_iter.max(1) {
        let r = residuals(&x);
        let m = r.len();
        let err: f64 = r.iter().map(|v| v * v).sum();
        if m == 0 || err < 1e-24 {
            break;
        }
        // Sparse Jacobian. The earlier scheme recomputed the whole residual vector `nv` times, O(nv·m), and
        // built a dense JᵀJ, O(nv²·m), on every iteration, which made dragging a large sketch lag. Now a
        // constraint is differentiated only with respect to its own variables and only its own rows are
        // recomputed, O(m·8); the regularisation, arm and drag rows are analytic; and JᵀJ is accumulated from
        // sparse outer products.
        //
        // The Jacobian is analytic: the derivative of every constraint is written out exactly in `con_jac`,
        // with no finite differences. The numerical scheme was limited by its step — at coordinates of
        // hundreds of millimetres the solution stalled with an error around 1e-5, so a 130 mm dimension came
        // out as −129.99900 and a cut left a film across a wall instead of an opening. The formulas are kept
        // honest by a test that compares each one against a numerical derivative for every constraint type
        // (`solver_jacobian.rs`); an error in a formula would otherwise be silent.
        let mut jrows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
        {
            let mut base = 0usize;
            let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
            for (ci, c) in cons.iter().enumerate() {
                let n = con_nrows[ci];
                if n > 0 {
                    rows.clear();
                    con_jac(c, &x, &x0, &idx, &ridx, &mut rows);
                    debug_assert_eq!(rows.len(), n, "constraint Jacobian has a different row count than its residuals");
                    for (t, row) in rows.iter().enumerate().take(n) {
                        jrows[base + t] = row.clone();
                    }
                }
                base += n;
            }
            for k in 0..nv {
                jrows[base + k].push((k, w_reg)); // regularisation: ∂/∂xk = w_reg
            }
            base += nv;
            for &(ip, iq, _) in &angle_arms {
                let (dx, dy) = (x[2 * ip] - x[2 * iq], x[2 * ip + 1] - x[2 * iq + 1]);
                let l = (dx * dx + dy * dy).sqrt().max(1e-12);
                jrows[base].push((2 * ip, w_len * dx / l));
                jrows[base].push((2 * ip + 1, w_len * dy / l));
                jrows[base].push((2 * iq, -w_len * dx / l));
                jrows[base].push((2 * iq + 1, -w_len * dy / l));
                base += 1;
            }
            if let Some((di, _, _)) = drag {
                jrows[base].push((2 * di, w_drag));
                jrows[base + 1].push((2 * di + 1, w_drag));
            }
        }
        let mut grad = vec![0.0; nv];
        for (rowi, row) in jrows.iter().enumerate() {
            for &(i, vi) in row {
                grad[i] += vi * r[rowi];
            }
        }
        let anchored = fixed_var.iter().filter(|f| **f).count();
        let dense = match algebra {
            Algebra::Dense => true,
            Algebra::Sparse => false,
            Algebra::BySize => nv - anchored <= DENSE_UP_TO,
        };
        let delta = match if dense { dense_step(&jrows, &grad, &fixed_var, lambda) } else { sparse_step(&jrows, &grad, &fixed_var, lambda) } {
            Some(d) => d,
            // The normal equations are singular — a rank deficiency of a degenerate configuration: coincident
            // points, zero lengths, collinearity. Rather than give up, raise the damping, which grows the
            // diagonal through `a[i][i] += lambda` until the system becomes solvable, and retry on the next
            // iteration. A `break` here left the sketch under-solved on any momentary degeneracy, which showed
            // up as jerks.
            None => {
                lambda = (lambda * 4.0).clamp(1e-12, 1e6); // the lower bound also lifts lambda back out of zero
                continue;
            }
        };
        let mut trial = x.clone();
        for i in 0..nv {
            trial[i] += delta[i];
        }
        let new_err: f64 = residuals(&trial).iter().map(|v| v * v).sum();
        // The stopping criterion is relative. A fixed threshold of 1e-9 on the step meant "no further to
        // go" regardless of the size of the sketch: on a part of hundreds of millimetres that is still
        // coarse, leaving the solution 2e-9 away from the dimension, while on a tiny part it would have
        // prevented stopping in time.
        let step = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
        if new_err < err {
            x = trial;
            lambda *= 0.5;
            let scale = x.iter().map(|v| v * v).sum::<f64>().sqrt().max(1.0);
            if step < 1e-14 * scale {
                break;
            }
        } else {
            // A STEP REFUSED BELOW THE THRESHOLD ENDS THE RUN as an accepted one does: x, J and r stay as they are,
            // the damping only grows, and a Levenberg-Marquardt step shrinks as the damping grows - whatever step is
            // taken next is shorter still and would end the run on its own. Running on, a circle with its diameter
            // to change took all 120 steps of the first stage (5 with this), an arc 120 (23): 80 us a part, 5.6 s on
            // 70 000 circles.
            let scale = x.iter().map(|v| v * v).sum::<f64>().sqrt().max(1.0);
            if step < 1e-14 * scale {
                break;
            }
            lambda = (lambda * 4.0).clamp(1e-12, 1e6);
        }
    }

    for (i, p) in points.iter_mut().enumerate() {
        p.x = x[2 * i];
        p.y = x[2 * i + 1];
    }
    for (j, rv) in radii.iter_mut().enumerate() {
        rv.value = x[np * 2 + j].max(0.001); // a radius stays positive
    }
    residuals_of(&x, &x0, &idx, &ridx, &anchor, &cons).iter().map(|v| v * v).sum::<f64>().sqrt()
}

/// One step of Levenberg-Marquardt on the dense normal matrix: `(JᵀJ + lambda·I) d = -Jᵀr` over the free unknowns,
/// `None` where the matrix is singular.
// THE INDEX IS THE MEANING: `a[i][i]` is the DIAGONAL of the normal matrix, see `solve_lm`.
#[allow(clippy::needless_range_loop)]
fn dense_step(jrows: &[Vec<(usize, f64)>], grad: &[f64], fixed_var: &[bool], lambda: f64) -> Option<Vec<f64>> {
    let nv = grad.len();
    let mut a = vec![vec![0.0; nv]; nv];
    for row in jrows {
        for &(i, vi) in row {
            for &(j, vj) in row {
                a[i][j] += vi * vj;
            }
        }
    }
    // Levenberg damping, plain λ·I. It is needed at the start so that the step does not fly off on
    // degenerate configurations; the accuracy of the final answer comes from the polish stage at λ = 0,
    // which is pure Gauss-Newton with quadratic convergence.
    //
    // Scaling the diagonal instead (the Marquardt form) was tried and dropped: along degenerate directions
    // the damping vanished, the solution no longer reached the minimum within the iteration budget, and the
    // next call carried on descending, so the sketch drifted between solves — the property test caught it.
    // Once the real causes of the accuracy loss were fixed (analytic Jacobian, hard anchoring, splitting the
    // pull towards the previous state into two stages), the scaled form bought nothing anyway.
    for i in 0..nv {
        a[i][i] += lambda;
    }
    // An anchored point is not a variable. `Fixed` used to be a penalty row of weight 50, so the anchor
    // held only softly: the sketch axis drifted by about 1e-9 during a solve and tilted slightly, and
    // everything measured from it inherited the drift — a 130 mm dimension produced the coordinate
    // −129.9999997 while the constraint itself was formally satisfied, the point being correct relative to
    // the axis that had moved. Anchored geometry is therefore excluded from the unknowns: the system is
    // solved over the free variables only, and anchored ones stay exactly where they were anchored.
    let free: Vec<usize> = (0..nv).filter(|k| !fixed_var[*k] && a[*k][*k] > 1e-12).collect();
    let ared: Vec<Vec<f64>> = free.iter().map(|&i| free.iter().map(|&j| a[i][j]).collect()).collect();
    let rhs: Vec<f64> = free.iter().map(|&i| -grad[i]).collect();
    let d = solve_linear(ared, rhs)?;
    let mut full = vec![0.0; nv];
    for (t, &k) in free.iter().enumerate() {
        full[k] = d[t];
    }
    Some(full)
}

/// The same step on the sparse normal matrix, by its Cholesky factor (`faer`, ordered to keep the factor sparse): the
/// same sums, the same damping and the same free unknowns as `dense_step`, at the cost of the non-zeros instead of
/// the cube of the unknowns. `None` where the matrix is not positive definite - singular, as the dense step says.
fn sparse_step(jrows: &[Vec<(usize, f64)>], grad: &[f64], fixed_var: &[bool], lambda: f64) -> Option<Vec<f64>> {
    use faer::linalg::solvers::Solve;
    use faer::sparse::{SparseColMat, Triplet};
    let nv = grad.len();
    // the lower triangle of JᵀJ + lambda·I, every product as the dense matrix sums it, then summed by place
    let mut parts: Vec<Triplet<usize, usize, f64>> = (0..nv).map(|k| Triplet::new(k, k, lambda)).collect();
    for row in jrows {
        for &(i, vi) in row {
            for &(j, vj) in row {
                if i >= j {
                    parts.push(Triplet::new(i, j, vi * vj));
                }
            }
        }
    }
    parts.sort_unstable_by_key(|e| (e.col, e.row));
    let mut sums: Vec<Triplet<usize, usize, f64>> = Vec::with_capacity(parts.len());
    for e in parts {
        match sums.last_mut() {
            Some(last) if last.row == e.row && last.col == e.col => last.val += e.val,
            _ => sums.push(e),
        }
    }
    let mut diagonal = vec![0.0; nv];
    for e in sums.iter().filter(|e| e.row == e.col) {
        diagonal[e.row] = e.val;
    }
    // the free unknowns as `dense_step` takes them, and the place of each among them
    let free: Vec<usize> = (0..nv).filter(|&k| !fixed_var[k] && diagonal[k] > 1e-12).collect();
    let mut place = vec![usize::MAX; nv];
    for (t, &k) in free.iter().enumerate() {
        place[k] = t;
    }
    let own: Vec<Triplet<usize, usize, f64>> = sums.iter().filter(|e| place[e.row] != usize::MAX && place[e.col] != usize::MAX).map(|e| Triplet::new(place[e.row], place[e.col], e.val)).collect();
    let n = free.len();
    let matrix = SparseColMat::<usize, f64>::try_new_from_triplets(n, n, &own).ok()?;
    let factor = matrix.sp_cholesky(faer::Side::Lower).ok()?;
    let d = factor.solve(faer::Mat::<f64>::from_fn(n, 1, |t, _| -grad[free[t]]));
    let mut full = vec![0.0; nv];
    for (t, &k) in free.iter().enumerate() {
        full[k] = d[(t, 0)];
    }
    full.iter().all(|v| v.is_finite()).then_some(full)
}

/// Direct solution of the linear system `a·x = b` by Gauss-Jordan elimination with partial pivoting.
// THE INDEX IS THE MEANING: Gaussian elimination subtracts row `col` from row `r` starting at column
// `col`. Rows and columns are what the method is about, and hiding them behind iterators hides the method.
#[allow(clippy::needless_range_loop)]
fn solve_linear(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = a.len();
    for col in 0..n {
        let mut piv = col;
        for r in col + 1..n {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        if a[piv][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, piv);
        b.swap(col, piv);
        let d = a[col][col];
        for r in 0..n {
            if r == col {
                continue;
            }
            let f = a[r][col] / d;
            if f != 0.0 {
                for c in col..n {
                    a[r][c] -= f * a[col][c];
                }
                b[r] -= f * b[col];
            }
        }
    }
    Some((0..n).map(|i| b[i] / a[i][i]).collect())
}

/// Residuals of every constraint at coordinates `x`; shared by the solve and by the degree-of-freedom analysis.
fn residuals_of(x: &[f64], x0: &[f64], idx: &HashMap<Id, usize>, ridx: &HashMap<Id, usize>, anchor: &HashMap<Id, (f64, f64)>, cons: &[Constraint]) -> Vec<f64> {
    let mut r = Vec::new();
    for c in cons {
        con_rows(c, x, x0, idx, ridx, anchor, &mut r);
    }
    r
}

/// Constraint diagnostics: the residual of each constraint separately, in the order of `constraints`.
///
/// A single residual for the whole sketch says only that something does not fit; finding out which constraints
/// disagree then means switching them off one at a time. Per-constraint residuals show it directly.
pub fn residual_per_constraint(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> Vec<f64> {
    let np = points.len();
    let idx: HashMap<Id, usize> = points.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
    let ridx: HashMap<Id, usize> = radii.iter().enumerate().map(|(j, rv)| (rv.center, np * 2 + j)).collect();
    let has = |id: Id| idx.contains_key(&id);
    let is_center = |id: Id| ridx.contains_key(&id);
    let anchor: HashMap<Id, (f64, f64)> = constraints
        .iter()
        .filter_map(|c| match *c {
            Constraint::Fixed { p } => idx.get(&p).map(|&i| (p, (points[i].x, points[i].y))),
            _ => None,
        })
        .collect();
    let mut x: Vec<f64> = points.iter().flat_map(|p| [p.x, p.y]).collect();
    x.extend(radii.iter().map(|rv| rv.value));
    constraints
        .iter()
        .map(|c| {
            if !cons_ok(c, &has, &is_center) {
                return 0.0;
            }
            let mut r = Vec::new();
            con_rows(c, &x, &x, &idx, &ridx, &anchor, &mut r);
            r.iter().map(|v| v * v).sum::<f64>().sqrt()
        })
        .collect()
}

/// Conflicting constraints: the indices of constraints that are contradictory *together* — the system cannot be
/// satisfied until at least one of them is removed or turned into a driven (reference) one.
///
/// The method: reduce the augmented matrix [J | r] to row echelon form. A row whose coefficients on the
/// variables have cancelled out while its residual has not is a linear combination of constraints with an
/// inconsistent right-hand side, and every constraint whose row entered that combination belongs to the
/// conflicting set. This is what lets the sketch point at the specific dimensions that disagree instead of
/// reporting one overall residual.
pub fn conflicts(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> Vec<usize> {
    // Part by part, as `dof`: no row of one part combines with a row of another. The rows of every part are judged
    // together, against the largest residual of the sketch, as the rows of the whole were.
    let mut rows = Vec::new();
    for part in parts(points, radii, constraints).iter().filter(|p| !p.constraints.is_empty()) {
        let own = part.own(points, radii, constraints);
        rows.extend(eliminated(&own.points, &own.radii, &own.constraints).into_iter().map(|r| Eliminated { from: r.from.iter().map(|&k| part.constraints[k]).collect(), ..r }));
    }
    conflicting(&rows)
}

/// The conflicting constraints of the whole sketch from one elimination (`conflicts`); the reference of the parts.
pub fn conflicts_whole(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> Vec<usize> {
    conflicting(&eliminated(points, radii, constraints))
}

/// A row of the augmented matrix [J | r] after the elimination of `eliminated`.
struct Eliminated {
    /// no coefficient is left on a free unknown: moving the unanchored geometry can no longer satisfy the row
    null: bool,
    residual: f64,
    /// the constraints whose rows went into it
    from: Vec<usize>,
}

/// The constraints of the null rows whose residual is left: those that disagree together.
fn conflicting(rows: &[Eliminated]) -> Vec<usize> {
    // residual scale: compare against a typical magnitude rather than against absolute zero
    let scale = rows.iter().map(|r| r.residual.abs()).fold(0.0_f64, f64::max).max(1.0);
    let mut bad: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for r in rows.iter().filter(|r| r.null && r.residual.abs() > 1e-6 * scale) {
        bad.extend(r.from.iter().copied());
    }
    bad.into_iter().collect()
}

/// The rows of [J | r] of a sketch reduced to row echelon form, see `conflicts`.
fn eliminated(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> Vec<Eliminated> {
    let np = points.len();
    if np == 0 {
        return Vec::new();
    }
    let idx: HashMap<Id, usize> = points.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
    let ridx: HashMap<Id, usize> = radii.iter().enumerate().map(|(j, rv)| (rv.center, np * 2 + j)).collect();
    let has = |id: Id| idx.contains_key(&id);
    let is_center = |id: Id| ridx.contains_key(&id);
    let anchor: HashMap<Id, (f64, f64)> = constraints
        .iter()
        .filter_map(|c| match *c {
            Constraint::Fixed { p } => idx.get(&p).map(|&i| (p, (points[i].x, points[i].y))),
            _ => None,
        })
        .collect();
    let mut x: Vec<f64> = points.iter().flat_map(|p| [p.x, p.y]).collect();
    x.extend(radii.iter().map(|rv| rv.value));
    let nv = np * 2 + radii.len();

    // Anchored points are not variables here either, exactly as in the solve. Otherwise the analysis treats
    // their coordinates as free, and a constraint that is impossible precisely because of an anchor — say a
    // tangency dimension between two pinned circles — is never recognised as contradictory.
    let mut fixed_var = vec![false; nv];
    for c in constraints.iter() {
        if let Constraint::Fixed { p } = *c {
            if let Some(&i) = idx.get(&p) {
                fixed_var[2 * i] = true;
                fixed_var[2 * i + 1] = true;
            }
        }
    }

    // rows: [coefficients per variable | residual | set of constraints that went into the row]
    let mut rows: Vec<(Vec<f64>, f64, Vec<usize>)> = Vec::new();
    for (ci, c) in constraints.iter().enumerate() {
        if !cons_ok(c, &has, &is_center) || matches!(c, Constraint::Fixed { .. }) {
            continue; // an anchor is part of the statement of the problem, not a dimension that can disagree
        }
        if c.is_driven() {
            continue; // a driven dimension constrains nothing
        }
        let mut r = Vec::new();
        con_rows(c, &x, &x, &idx, &ridx, &anchor, &mut r);
        let mut j = Vec::new();
        con_jac(c, &x, &x, &idx, &ridx, &mut j);
        for (t, rv) in r.iter().enumerate() {
            let mut dense = vec![0.0; nv];
            if let Some(row) = j.get(t) {
                for &(k, v) in row {
                    dense[k] += v;
                }
            }
            rows.push((dense, *rv, vec![ci]));
        }
    }

    // forward elimination with pivoting
    let mut used = vec![false; rows.len()];
    for col in (0..nv).filter(|k| !fixed_var[*k]) {
        let piv = (0..rows.len()).filter(|&i| !used[i]).max_by(|&a, &b| rows[a].0[col].abs().partial_cmp(&rows[b].0[col].abs()).unwrap_or(std::cmp::Ordering::Equal));
        let Some(piv) = piv.filter(|&i| rows[i].0[col].abs() > 1e-9) else { continue };
        used[piv] = true;
        for i in 0..rows.len() {
            if i == piv || used[i] || rows[i].0[col].abs() <= 1e-12 {
                continue;
            }
            let f = rows[i].0[col] / rows[piv].0[col];
            for k in col..nv {
                rows[i].0[k] -= f * rows[piv].0[k];
            }
            rows[i].1 -= f * rows[piv].1;
            let src: Vec<usize> = rows[piv].2.clone();
            rows[i].2.extend(src);
        }
    }
    rows.into_iter()
        .map(|(dense, residual, from)| {
            // a row counts as null when no coefficients are left on the free variables: moving the unanchored
            // geometry can no longer satisfy it
            let null = (0..nv).filter(|k| !fixed_var[*k]).all(|k| dense[k].abs() <= 1e-7);
            Eliminated { null, residual, from }
        })
        .collect()
}

/// Jacobian cross-check, used by tests: the largest discrepancy between the analytic derivatives of a
/// constraint and numerical ones.
///
/// An error in a hand-written formula is the quietest failure available — the solution simply converges a
/// little worse, which goes unnoticed until a part shows up where a 130 mm dimension lands at −129.99900. Every
/// constraint type is therefore checked here against a central difference on a random, non-degenerate
/// configuration.
pub fn jacobian_mismatch(points: &[SketchPoint], radii: &[RadiusVar], c: &Constraint) -> f64 {
    let idx: HashMap<Id, usize> = points.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
    let np = points.len();
    let ridx: HashMap<Id, usize> = radii.iter().enumerate().map(|(i, r)| (r.center, np * 2 + i)).collect();
    let anchor: HashMap<Id, (f64, f64)> = points.iter().map(|p| (p.id, (p.x, p.y))).collect();
    let mut x: Vec<f64> = points.iter().flat_map(|p| [p.x, p.y]).collect();
    x.extend(radii.iter().map(|r| r.value));
    let x0 = x.clone();

    let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
    con_jac(c, &x, &x0, &idx, &ridx, &mut rows);
    let r_at = |xx: &[f64]| -> Vec<f64> {
        let mut r = Vec::new();
        con_rows(c, xx, &x0, &idx, &ridx, &anchor, &mut r);
        r
    };
    let base = r_at(&x);
    let mut worst = 0.0_f64;
    for k in 0..x.len() {
        let h = 1e-6 * x[k].abs().max(1.0);
        let mut xp = x.clone();
        xp[k] = x[k] + h;
        let rp = r_at(&xp);
        xp[k] = x[k] - h;
        let rm = r_at(&xp);
        for t in 0..base.len() {
            let num = (rp[t] - rm[t]) / (2.0 * h);
            let ana: f64 = rows.get(t).map(|row| row.iter().filter(|(i, _)| *i == k).map(|(_, v)| *v).sum()).unwrap_or(0.0);
            let scale = num.abs().max(ana.abs()).max(1.0);
            worst = worst.max((num - ana).abs() / scale);
        }
    }
    worst
}

/// Analytic Jacobian of a single constraint: the exact derivatives of every residual row with respect to its own
/// variables.
///
/// The Jacobian used to be computed by finite differences, and that scheme has two flaws. Accuracy is limited by
/// the step: at coordinates of hundreds of millimetres the solution stalled with an error around 1e-5, so a
/// 130 mm dimension came out as −129.99900 and a cut left a film instead of an opening. And the cost is an extra
/// residual evaluation per variable per iteration. With the derivatives written out, the solution settles onto a
/// dimension to machine precision and exactly as many rows are computed as are needed.
///
/// Format: `out[k]` is row k of this constraint, a list of `(variable index, ∂r_k/∂x)`. The row order matches
/// `con_rows` exactly, which the numerical cross-check test verifies for every type.
fn con_jac(c: &Constraint, x: &[f64], x0: &[f64], idx: &HashMap<Id, usize>, ridx: &HashMap<Id, usize>, out: &mut Vec<Vec<(usize, f64)>>) {
    let g = |id: Id| -> (f64, f64) {
        let i = idx[&id];
        (x[2 * i], x[2 * i + 1])
    };
    let vx = |id: Id| -> usize { 2 * idx[&id] };
    let vy = |id: Id| -> usize { 2 * idx[&id] + 1 };
    let rv = |id: Id| -> Option<usize> { ridx.get(&id).copied() };
    // derivatives of the distance |P−Q| with respect to the coordinates of P and Q
    let dist_rows = |p: Id, q: Id, sign: f64, row: &mut Vec<(usize, f64)>| {
        let ((px, py), (qx, qy)) = (g(p), g(q));
        let (dx, dy) = (px - qx, py - qy);
        let l = (dx * dx + dy * dy).sqrt().max(1e-12);
        row.push((vx(p), sign * dx / l));
        row.push((vy(p), sign * dy / l));
        row.push((vx(q), -sign * dx / l));
        row.push((vy(q), -sign * dy / l));
    };

    match *c {
        Constraint::Fixed { p } => {
            const W_FIX: f64 = 50.0;
            out.push(vec![(vx(p), W_FIX)]);
            out.push(vec![(vy(p), W_FIX)]);
        }
        Constraint::Horizontal { a, b } => out.push(vec![(vy(a), 1.0), (vy(b), -1.0)]),
        Constraint::Vertical { a, b } => out.push(vec![(vx(a), 1.0), (vx(b), -1.0)]),
        Constraint::Orientation { a, b, deg } => {
            let (sn, cs) = deg.to_radians().sin_cos();
            out.push(vec![(vx(b), sn), (vy(b), -cs), (vx(a), -sn), (vy(a), cs)]);
        }
        Constraint::Coincident { a, b } | Constraint::Concentric { c1: a, c2: b } => {
            out.push(vec![(vx(a), 1.0), (vx(b), -1.0)]);
            out.push(vec![(vy(a), 1.0), (vy(b), -1.0)]);
        }
        Constraint::Distance { a, b, axis, .. } => match axis {
            1 | 2 => {
                let k = if axis == 1 { 0 } else { 1 };
                let (ia, ib) = (idx[&a], idx[&b]);
                let cur = x[2 * ia + k] - x[2 * ib + k];
                let d0 = x0[2 * ia + k] - x0[2 * ib + k];
                // |Δ| − d away from the degenerate case, where the derivative is the sign of Δ; signed Δ − d at
                // zero, see `con_rows`
                let s = if d0.abs() > 1e-9 {
                    if cur < 0.0 {
                        -1.0
                    } else {
                        1.0
                    }
                } else {
                    1.0
                };
                out.push(vec![(2 * ia + k, s), (2 * ib + k, -s)]);
            }
            _ => {
                let mut row = Vec::new();
                dist_rows(a, b, 1.0, &mut row);
                out.push(row);
            }
        },
        Constraint::EdgeDistance { c1, c2, m1, m2, .. } => {
            let mut row = Vec::new();
            dist_rows(c2, c1, 1.0, &mut row); // dist = |P2−P1|
            if let Some(i) = rv(c1) {
                row.push((i, m1 as f64));
            }
            if let Some(i) = rv(c2) {
                row.push((i, m2 as f64));
            }
            out.push(row);
        }
        Constraint::Parallel { a, b, c: cc, d } => {
            let ((ax, ay), (bx, by)) = (g(a), g(b));
            let ((cx, cy), (dx, dy)) = (g(cc), g(d));
            let (ux, uy) = (bx - ax, by - ay);
            let (wx, wy) = (dx - cx, dy - cy);
            out.push(vec![(vx(a), -wy), (vy(a), wx), (vx(b), wy), (vy(b), -wx), (vx(cc), uy), (vy(cc), -ux), (vx(d), -uy), (vy(d), ux)]);
        }
        Constraint::Perpendicular { a, b, c: cc, d } => {
            let ((ax, ay), (bx, by)) = (g(a), g(b));
            let ((cx, cy), (dx, dy)) = (g(cc), g(d));
            let (ux, uy) = (bx - ax, by - ay);
            let (wx, wy) = (dx - cx, dy - cy);
            out.push(vec![(vx(a), -wx), (vy(a), -wy), (vx(b), wx), (vy(b), wy), (vx(cc), -ux), (vy(cc), -uy), (vx(d), ux), (vy(d), uy)]);
        }
        Constraint::Equal { a, b, c: cc, d } => {
            let mut row = Vec::new();
            dist_rows(a, b, 1.0, &mut row);
            dist_rows(cc, d, -1.0, &mut row);
            out.push(row);
        }
        // two points on a line, the rows of `PointOnLine` for each end of the second segment - see the residual
        Constraint::Collinear { a, b, c: cc, d } => {
            let ((ax, ay), (bx, by)) = (g(a), g(b));
            let (dx, dy) = (bx - ax, by - ay);
            let len = (dx * dx + dy * dy).sqrt().max(1e-9);
            for far in [cc, d] {
                let (px, py) = g(far);
                let cross = dx * (py - ay) - dy * (px - ax);
                let dc = [(vx(far), -dy), (vy(far), dx), (vx(a), -(py - ay) + dy), (vy(a), -dx + (px - ax)), (vx(b), py - ay), (vy(b), -(px - ax))];
                let dl = [(vx(a), -dx / len), (vy(a), -dy / len), (vx(b), dx / len), (vy(b), dy / len)];
                let mut row: Vec<(usize, f64)> = dc.iter().map(|&(i, v)| (i, v / len)).collect();
                for &(i, v) in &dl {
                    row.push((i, -cross * v / (len * len)));
                }
                out.push(row);
            }
        }
        Constraint::Midpoint { p, a, b } => {
            out.push(vec![(vx(p), 1.0), (vx(a), -0.5), (vx(b), -0.5)]);
            out.push(vec![(vy(p), 1.0), (vy(a), -0.5), (vy(b), -0.5)]);
        }
        Constraint::Tangent { a, b, c: cc, .. } => {
            let ((ax, ay), (bx, by), (cx, cy)) = (g(a), g(b), g(cc));
            let (dx, dy) = (bx - ax, by - ay);
            let len = (dx * dx + dy * dy).sqrt().max(1e-9);
            let cross = dx * (cy - ay) - dy * (cx - ax);
            let s = if cross < 0.0 { -1.0 } else { 1.0 };
            // f = s·cross/len  →  ∂f = s·(∂cross/len − cross·∂len/len²)
            let dc = [(vx(a), -(cy - ay) + dy), (vy(a), -dx + (cx - ax)), (vx(b), cy - ay), (vy(b), -(cx - ax)), (vx(cc), -dy), (vy(cc), dx)];
            let dl = [(vx(a), -dx / len), (vy(a), -dy / len), (vx(b), dx / len), (vy(b), dy / len)];
            let mut row: Vec<(usize, f64)> = dc.iter().map(|&(i, v)| (i, s * v / len)).collect();
            for &(i, v) in &dl {
                row.push((i, -s * cross * v / (len * len)));
            }
            if let Some(i) = rv(cc) {
                row.push((i, -1.0));
            }
            out.push(row);
        }
        Constraint::CircleTangent { c1, c2, external } => {
            let mut row = Vec::new();
            dist_rows(c2, c1, 1.0, &mut row);
            let (r1, r2) = (rv(c1).map(|i| x[i]).unwrap_or(0.0), rv(c2).map(|i| x[i]).unwrap_or(0.0));
            let (g1, g2) = if external {
                (-1.0, -1.0)
            } else {
                let s = if r1 - r2 < 0.0 { -1.0 } else { 1.0 };
                (-s, s)
            };
            if let Some(i) = rv(c1) {
                row.push((i, g1));
            }
            if let Some(i) = rv(c2) {
                row.push((i, g2));
            }
            out.push(row);
        }
        Constraint::Symmetric { a, b, la, lb } => {
            let ((ax, ay), (bx, by)) = (g(a), g(b));
            let ((lax, lay), (lbx, lby)) = (g(la), g(lb));
            let (dx, dy) = (lbx - lax, lby - lay);
            let (mx, my) = (0.5 * (ax + bx), 0.5 * (ay + by));
            out.push(vec![
                (vx(a), -dy * 0.5),
                (vy(a), dx * 0.5),
                (vx(b), -dy * 0.5),
                (vy(b), dx * 0.5),
                (vx(la), -(my - lay) + dy),
                (vy(la), (mx - lax) - dx),
                (vx(lb), my - lay),
                (vy(lb), -(mx - lax)),
            ]);
            out.push(vec![(vx(a), -dx), (vy(a), -dy), (vx(b), dx), (vy(b), dy), (vx(la), -(bx - ax)), (vy(la), -(by - ay)), (vx(lb), bx - ax), (vy(lb), by - ay)]);
        }
        Constraint::Angle { a, b, c: cc, .. } => {
            let ((ax, ay), (bx, by), (cx, cy)) = (g(a), g(b), g(cc));
            let (ux, uy) = (ax - bx, ay - by);
            let (wx, wy) = (cx - bx, cy - by);
            let cross = ux * wy - uy * wx;
            let dot = ux * wx + uy * wy;
            let s = if cross < 0.0 { -1.0 } else { 1.0 };
            let den = (cross * cross + dot * dot).max(1e-18);
            // θ = atan2(|cross|, dot): ∂θ/∂cross = s·dot/den, ∂θ/∂dot = −|cross|/den
            let (kc, kd) = (s * dot / den, -cross.abs() / den);
            let dcross = [(vx(a), wy), (vy(a), -wx), (vx(cc), -uy), (vy(cc), ux), (vx(b), -wy + uy), (vy(b), wx - ux)];
            let ddot = [(vx(a), wx), (vy(a), wy), (vx(cc), ux), (vy(cc), uy), (vx(b), -wx - ux), (vy(b), -wy - uy)];
            let mut row: Vec<(usize, f64)> = dcross.iter().map(|&(i, v)| (i, kc * v)).collect();
            for &(i, v) in &ddot {
                row.push((i, kd * v));
            }
            out.push(row);
        }
        Constraint::AngleLines { a, b, c: cc, d, .. } => {
            let ((ax, ay), (bx, by)) = (g(a), g(b));
            let ((cx, cy), (dx, dy)) = (g(cc), g(d));
            let (ux, uy) = (bx - ax, by - ay);
            let (wx, wy) = (dx - cx, dy - cy);
            let cross = ux * wy - uy * wx;
            let dot = ux * wx + uy * wy;
            let s = if cross < 0.0 { -1.0 } else { 1.0 };
            let den = (cross * cross + dot * dot).max(1e-18);
            let (kc, kd) = (s * dot / den, -cross.abs() / den);
            let dcross = [(vx(a), -wy), (vy(a), wx), (vx(b), wy), (vy(b), -wx), (vx(cc), uy), (vy(cc), -ux), (vx(d), -uy), (vy(d), ux)];
            let ddot = [(vx(a), -wx), (vy(a), -wy), (vx(b), wx), (vy(b), wy), (vx(cc), -ux), (vy(cc), -uy), (vx(d), ux), (vy(d), uy)];
            let mut row: Vec<(usize, f64)> = dcross.iter().map(|&(i, v)| (i, kc * v)).collect();
            for &(i, v) in &ddot {
                row.push((i, kd * v));
            }
            out.push(row);
        }
        Constraint::PointOnLine { p, a, b } | Constraint::DistancePL { p, a, b, .. } => {
            let ((px, py), (ax, ay), (bx, by)) = (g(p), g(a), g(b));
            let (dx, dy) = (bx - ax, by - ay);
            let len = (dx * dx + dy * dy).sqrt().max(1e-9);
            let cross = dx * (py - ay) - dy * (px - ax);
            let dc = [(vx(p), -dy), (vy(p), dx), (vx(a), -(py - ay) + dy), (vy(a), -dx + (px - ax)), (vx(b), py - ay), (vy(b), -(px - ax))];
            let dl = [(vx(a), -dx / len), (vy(a), -dy / len), (vx(b), dx / len), (vy(b), dy / len)];
            let mut row: Vec<(usize, f64)> = dc.iter().map(|&(i, v)| (i, v / len)).collect();
            for &(i, v) in &dl {
                row.push((i, -cross * v / (len * len)));
            }
            out.push(row);
        }
        Constraint::Diameter { c: cc, .. } => {
            if let Some(i) = rv(cc) {
                out.push(vec![(i, 1.0)]);
            }
        }
        Constraint::EqualRadius { c1, c2 } => {
            if let (Some(i1), Some(i2)) = (rv(c1), rv(c2)) {
                out.push(vec![(i1, 1.0), (i2, -1.0)]);
            }
        }
        Constraint::PointOnCircle { p, c: cc } => {
            let mut row = Vec::new();
            dist_rows(p, cc, 1.0, &mut row);
            if let Some(i) = rv(cc) {
                row.push((i, -1.0));
            }
            out.push(row);
        }
        Constraint::ArcLength { c: cc, a, b, ccw, .. } => {
            let ((cx, cy), (ax, ay), (bx, by)) = (g(cc), g(a), g(b));
            let (rx, ry) = (ax - cx, ay - cy);
            let rad = (rx * rx + ry * ry).sqrt().max(1e-12);
            let (a0, a1) = ((ay - cy).atan2(ax - cx), (by - cy).atan2(bx - cx));
            let theta = if ccw { (a1 - a0).rem_euclid(std::f64::consts::TAU) } else { (a0 - a1).rem_euclid(std::f64::consts::TAU) };
            let sgn = if ccw { 1.0 } else { -1.0 };
            // r = R·θ − len; ∂R with respect to (a, c); ∂θ through ∂atan2 at both endpoints
            let (bx0, by0) = (bx - cx, by - cy);
            let rb2 = (bx0 * bx0 + by0 * by0).max(1e-18);
            let ra2 = (rx * rx + ry * ry).max(1e-18);
            out.push(vec![
                (vx(a), theta * rx / rad + rad * (-sgn) * (-ry / ra2)),
                (vy(a), theta * ry / rad + rad * (-sgn) * (rx / ra2)),
                (vx(b), rad * sgn * (-by0 / rb2)),
                (vy(b), rad * sgn * (bx0 / rb2)),
                (vx(cc), -theta * rx / rad + rad * (sgn * (-ry / ra2) - sgn * (-by0 / rb2))),
                (vy(cc), -theta * ry / rad + rad * (sgn * (rx / ra2) - sgn * (bx0 / rb2))),
            ]);
        }
    }
}

/// Residuals of a single constraint, one or two rows, appended to `r`. One body of code serves both the full
/// residual vector and the sparse Jacobian: a constraint touches at most eight variables, so only those and only
/// its own rows are differentiated.
fn con_rows(c: &Constraint, x: &[f64], x0: &[f64], idx: &HashMap<Id, usize>, ridx: &HashMap<Id, usize>, anchor: &HashMap<Id, (f64, f64)>, r: &mut Vec<f64>) {
    let g = |id: Id| -> (f64, f64) {
        let i = idx[&id];
        (x[2 * i], x[2 * i + 1])
    };
    // radius of the circle centred at `id` as a solver variable, when there is one
    let gr = |id: Id| -> Option<f64> { ridx.get(&id).map(|&i| x[i]) };
    {
        match *c {
            Constraint::Fixed { p } => {
                // A hard anchor: the large weight keeps an anchored point from drifting while the geometry is
                // edited and from yielding to the drag residual. Scaling a row does not change the rank, so
                // the degree-of-freedom count stays correct.
                const W_FIX: f64 = 50.0;
                let (px, py) = g(p);
                let (ax, ay) = anchor[&p];
                r.push(W_FIX * (px - ax));
                r.push(W_FIX * (py - ay));
            }
            Constraint::Horizontal { a, b } => r.push(g(a).1 - g(b).1),
            Constraint::Vertical { a, b } => r.push(g(a).0 - g(b).0),
            Constraint::Orientation { a, b, deg } => {
                // the side crossed with the direction held: linear in the points, and at 0 deg exactly a Horizontal. An
                // angle residual (atan2) has a slope of 1 / length^2 and let a side squeezed short by contradicting
                // constraints drift a point by 7.6 mm from one solve to the next.
                let ((ax, ay), (bx, by)) = (g(a), g(b));
                let (sn, cs) = deg.to_radians().sin_cos();
                r.push((bx - ax) * sn - (by - ay) * cs);
            }
            Constraint::Coincident { a, b } => {
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                r.push(ax - bx);
                r.push(ay - by);
            }
            Constraint::Distance { a, b, d, axis, .. } => {
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                match axis {
                    // The kink of |Δ| at zero. Away from zero the absolute value is smooth and keeps both
                    // sides of the solution available, which matters: a chain of dimensions may require
                    // flipping a side, and a signed residual would then freeze a perfectly solvable system
                    // into a least-squares compromise. Exactly at zero, however, there is no derivative at
                    // all, and the dimension used to be pulled out only by an accident of the forward
                    // difference, where h > 0 produced +1 where no honest derivative exists; switching to a
                    // central difference would have zeroed the row and the dimension would have stopped
                    // working silently. Hence: a non-degenerate start keeps |Δ| − d, while a degenerate one —
                    // a point collapsed onto a point, or a strictly vertical segment — uses the signed
                    // residual Δ − d, with a deterministic positive direction and a real derivative of 1.
                    1 | 2 => {
                        let (ia, ib) = (idx[&a], idx[&b]);
                        let k = if axis == 1 { 0 } else { 1 };
                        let cur = if k == 0 { ax - bx } else { ay - by };
                        let d0 = x0[2 * ia + k] - x0[2 * ib + k];
                        if d0.abs() > 1e-9 {
                            r.push(cur.abs() - d);
                        } else {
                            r.push(cur - d);
                        }
                    }
                    _ => r.push(((ax - bx).powi(2) + (ay - by).powi(2)).sqrt() - d), // aligned distance
                }
            }
            Constraint::EdgeDistance { c1, c2, d, m1, m2, .. } => {
                // distance between centres ± the radii, i.e. measured to the rim: a tangent dimension
                let (x1, y1) = g(c1);
                let (x2, y2) = g(c2);
                let dist = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
                let off = m1 as f64 * gr(c1).unwrap_or(0.0) + m2 as f64 * gr(c2).unwrap_or(0.0);
                r.push(dist + off - d);
            }
            Constraint::Parallel { a, b, c, d } => {
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (cx, cy) = g(c);
                let (dx, dy) = g(d);
                r.push((bx - ax) * (dy - cy) - (by - ay) * (dx - cx));
            }
            Constraint::Perpendicular { a, b, c, d } => {
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (cx, cy) = g(c);
                let (dx, dy) = g(d);
                r.push((bx - ax) * (dx - cx) + (by - ay) * (dy - cy));
            }
            Constraint::Equal { a, b, c, d } => {
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (cx, cy) = g(c);
                let (dx, dy) = g(d);
                let l1 = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
                let l2 = ((cx - dx).powi(2) + (cy - dy).powi(2)).sqrt();
                r.push(l1 - l2);
            }
            // BOTH ENDS OF THE SECOND SEGMENT LIE ON THE LINE OF THE FIRST, each as a distance - the cross product divided by
            // the length, as `PointOnLine` has it. Undivided, the residual was met by a first segment of no length: two
            // lines made collinear came out with the first shrunk to a point and the residual 0.
            Constraint::Collinear { a, b, c, d } => {
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (ux, uy) = (bx - ax, by - ay);
                let len = (ux * ux + uy * uy).sqrt().max(1e-9);
                for (px, py) in [g(c), g(d)] {
                    r.push((ux * (py - ay) - uy * (px - ax)) / len);
                }
            }
            Constraint::Midpoint { p, a, b } => {
                let (px, py) = g(p);
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                r.push(px - 0.5 * (ax + bx));
                r.push(py - 0.5 * (ay + by));
            }
            Constraint::Tangent { a, b, c, r: rad } => {
                // the line a→b touches the circle centred at c: distance(c, line) = radius. The radius is the
                // circle's own solver variable when it has one, otherwise the fixed `rad` (an arc, or a
                // fallback).
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (cx, cy) = g(c);
                let (dx, dy) = (bx - ax, by - ay);
                let len = (dx * dx + dy * dy).sqrt().max(1e-9);
                let dist = ((dx * (cy - ay) - dy * (cx - ax)) / len).abs();
                r.push(dist - gr(c).unwrap_or(rad));
            }
            Constraint::CircleTangent { c1, c2, external } => {
                // tangency of two circles: centre distance = r1 + r2 for external, |r1 − r2| for internal
                let (x1, y1) = g(c1);
                let (x2, y2) = g(c2);
                let d = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
                let (r1, r2) = (gr(c1).unwrap_or(0.0), gr(c2).unwrap_or(0.0));
                let target = if external { r1 + r2 } else { (r1 - r2).abs() };
                r.push(d - target);
            }
            Constraint::Symmetric { a, b, la, lb } => {
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (lax, lay) = g(la);
                let (lbx, lby) = g(lb);
                let (dx, dy) = (lbx - lax, lby - lay);
                let (mx, my) = (0.5 * (ax + bx), 0.5 * (ay + by));
                r.push(dx * (my - lay) - dy * (mx - lax));
                r.push((bx - ax) * dx + (by - ay) * dy);
            }
            Constraint::Angle { a, b, c, deg, .. } => {
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (cx, cy) = g(c);
                let (ux, uy) = (ax - bx, ay - by);
                let (vx, vy) = (cx - bx, cy - by);
                // Length-independent angular residual: θ(actual) − θ(target), with θ = atan2(|u×v|, u·v) in
                // [0, π]. The radial component of the gradient is zero, so this only rotates and the lengths
                // are held by `w_len`. A cosine residual was used before, and its gradient dies at 0° and
                // 180°: the solver stopped one or two degrees short of the target, resolving 179° as 177.3°.
                // The atan2 form keeps |∂r/∂θ| = 1 across the whole range and is sign-agnostic through
                // |cross|, so it does not mirror sides.
                let cross = ux * vy - uy * vx;
                let dot = ux * vx + uy * vy;
                r.push(cross.abs().atan2(dot) - deg.to_radians());
            }
            Constraint::PointOnLine { p, a, b } => {
                // signed perpendicular distance from point p to the line a→b equals zero
                let (px, py) = g(p);
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (dx, dy) = (bx - ax, by - ay);
                let len = (dx * dx + dy * dy).sqrt().max(1e-9);
                r.push((dx * (py - ay) - dy * (px - ax)) / len);
            }
            Constraint::DistancePL { p, a, b, d, .. } => {
                // signed perpendicular distance from point p to the line a→b equals d. Keeping d signed
                // preserves the side, so the point is not mirrored across the line during a solve.
                let (px, py) = g(p);
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (dx, dy) = (bx - ax, by - ay);
                let len = (dx * dx + dy * dy).sqrt().max(1e-9);
                let perp = (dx * (py - ay) - dy * (px - ax)) / len;
                r.push(perp - d);
            }
            Constraint::Diameter { c, d, diam, .. } => {
                // the circle's radius variable equals the given value (a diameter halves to r = d/2); it
                // counts as a real constraint in the degree-of-freedom analysis
                if let Some(rv) = gr(c) {
                    r.push(rv - if diam { d * 0.5 } else { d });
                }
            }
            Constraint::EqualRadius { c1, c2 } => {
                if let (Some(r1), Some(r2)) = (gr(c1), gr(c2)) {
                    r.push(r1 - r2);
                }
            }
            Constraint::PointOnCircle { p, c } => {
                // point p lies on the circle centred at c: distance(p, c) equals the radius variable of c.
                // This is what keeps the endpoints of an arc intrinsically on its own circle, making the arc
                // a real one.
                let (px, py) = g(p);
                let (cx, cy) = g(c);
                let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                r.push(d - gr(c).unwrap_or(d));
            }
            Constraint::Concentric { c1, c2 } => {
                let (ax, ay) = g(c1);
                let (bx, by) = g(c2);
                r.push(ax - bx);
                r.push(ay - by);
            }
            Constraint::ArcLength { c, a, b, ccw, len, .. } => {
                // arc length = R·θ, with R = |c→a| and θ the angle swept from a to b in the `ccw` direction
                let (cx, cy) = g(c);
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let rad = ((ax - cx).powi(2) + (ay - cy).powi(2)).sqrt();
                let (a0, a1) = ((ay - cy).atan2(ax - cx), (by - cy).atan2(bx - cx));
                let theta = if ccw { (a1 - a0).rem_euclid(std::f64::consts::TAU) } else { (a0 - a1).rem_euclid(std::f64::consts::TAU) };
                r.push(rad * theta - len);
            }
            Constraint::AngleLines { a, b, c, d, deg, .. } => {
                // angle between the directions of the segments a→b and c→d
                let (ax, ay) = g(a);
                let (bx, by) = g(b);
                let (cx, cy) = g(c);
                let (dx, dy) = g(d);
                let (ux, uy) = (bx - ax, by - ay);
                let (vx, vy) = (dx - cx, dy - cy);
                // atan2 residual, as in `Angle`: θ − θ₀ instead of a cosine residual, so the gradient does
                // not die at 0° and 180°. Length-independent and sign-agnostic; the lengths are held by
                // `w_len`.
                let cross = ux * vy - uy * vx;
                let dot = ux * vx + uy * vy;
                r.push(cross.abs().atan2(dot) - deg.to_radians());
            }
        }
    }
}

/// Whether a constraint can be evaluated at all. It is not enough that its points exist — it must also have
/// something to measure against.
///
/// `PointOnCircle { p, c }` needs the radius of the circle centred at `c`. If `c` is not a centre, as in a
/// damaged file or under an outside caller, there is no radius, the residual degenerates into "distance minus
/// the same distance" = 0, and the constraint becomes a silent no-op: the point looks constrained, the solver
/// does not see it, and the degree-of-freedom count does not drop. Rejecting it outright is better than
/// accepting it and not counting it.
fn cons_ok(c: &Constraint, has: &impl Fn(Id) -> bool, is_center: &impl Fn(Id) -> bool) -> bool {
    match *c {
        Constraint::Fixed { p } => has(p),
        Constraint::Horizontal { a, b } | Constraint::Vertical { a, b } | Constraint::Orientation { a, b, .. } | Constraint::Coincident { a, b } | Constraint::Distance { a, b, .. } => {
            has(a) && has(b)
        }
        Constraint::Parallel { a, b, c, d } | Constraint::Perpendicular { a, b, c, d } | Constraint::Equal { a, b, c, d } | Constraint::Collinear { a, b, c, d } => {
            has(a) && has(b) && has(c) && has(d)
        }
        Constraint::Angle { a, b, c, .. } => has(a) && has(b) && has(c),
        Constraint::Midpoint { p, a, b } => has(p) && has(a) && has(b),
        Constraint::Tangent { a, b, c, .. } => has(a) && has(b) && has(c),
        Constraint::Symmetric { a, b, la, lb } => has(a) && has(b) && has(la) && has(lb),
        Constraint::PointOnLine { p, a, b } => has(p) && has(a) && has(b),
        Constraint::DistancePL { p, a, b, .. } => has(p) && has(a) && has(b),
        Constraint::EdgeDistance { c1, c2, .. } => has(c1) && has(c2),
        Constraint::Diameter { c, .. } => has(c),
        Constraint::EqualRadius { c1, c2 } => has(c1) && has(c2),
        Constraint::CircleTangent { c1, c2, .. } => has(c1) && has(c2),
        Constraint::PointOnCircle { p, c } => has(p) && has(c) && is_center(c),
        Constraint::Concentric { c1, c2 } => has(c1) && has(c2),
        Constraint::ArcLength { c, a, b, .. } => has(c) && has(a) && has(b),
        Constraint::AngleLines { a, b, c, d, .. } => has(a) && has(b) && has(c) && has(d),
    }
}

/// Degrees of freedom of a sketch, from the rank of the constraint Jacobian; this accounts for redundancy and
/// for radius variables. Returns (degrees of freedom, number of redundant constraint equations).
pub fn dof(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> (i32, i32) {
    // The rank of a Jacobian whose parts share no unknown is the sum of the ranks of the parts: a sketch is counted
    // part by part, and a part with no constraint is all freedom. 10 000 separate lines took 49 s counted whole.
    let parts = parts(points, radii, constraints);
    let mut free = (radii.len() - parts.iter().map(|p| p.radii.len()).sum::<usize>()) as i32; // radii of no point
    let mut redundant = 0;
    for part in &parts {
        if part.constraints.is_empty() {
            free += (part.points.len() * 2 + part.radii.len()) as i32;
            continue;
        }
        let own = part.own(points, radii, constraints);
        let (f, r) = dof_whole(&own.points, &own.radii, &own.constraints);
        free += f;
        redundant += r;
    }
    (free, redundant)
}

/// The degrees of freedom of the whole sketch counted as one Jacobian, the way they were counted before the sketch
/// was split into parts (`dof`); the reference of the parts.
pub fn dof_whole(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> (i32, i32) {
    if points.is_empty() {
        return (0, 0);
    }
    let np = points.len();
    let idx: HashMap<Id, usize> = points.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
    let ridx: HashMap<Id, usize> = radii.iter().enumerate().map(|(j, rv)| (rv.center, np * 2 + j)).collect();
    let has = |id: Id| idx.contains_key(&id);
    let is_center = |id: Id| ridx.contains_key(&id);
    let cons: Vec<Constraint> = constraints.iter().filter(|&c| cons_ok(c, &has, &is_center)).cloned().collect();
    let nv = np * 2 + radii.len();
    let anchor: HashMap<Id, (f64, f64)> = cons
        .iter()
        .filter_map(|c| match *c {
            Constraint::Fixed { p } => idx.get(&p).map(|&i| (p, (points[i].x, points[i].y))),
            _ => None,
        })
        .collect();
    let mut x: Vec<f64> = points.iter().flat_map(|p| [p.x, p.y]).collect();
    x.extend(radii.iter().map(|rv| rv.value));
    let r0 = residuals_of(&x, &x, &idx, &ridx, &anchor, &cons); // the side of an axis dimension is read off the current configuration
    let m = r0.len();
    if m == 0 {
        return (nv as i32, 0);
    }
    let h = 1e-6;
    let mut jac = vec![vec![0.0_f64; nv]; m];
    for k in 0..nv {
        let mut xp = x.clone();
        xp[k] += h;
        let rp = residuals_of(&xp, &x, &idx, &ridx, &anchor, &cons);
        for i in 0..m {
            jac[i][k] = (rp[i] - r0[i]) / h;
        }
    }
    normalize_rows(&mut jac);
    let rank = pivot_columns(&mut jac, nv).len() as i32;
    let dof = nv as i32 - rank;
    let redundant = m as i32 - rank;
    (dof, redundant)
}

/// Normalise the rows of the Jacobian to unit length before taking the rank. The pivot threshold (1e-7) is
/// absolute, while the magnitudes of the derivatives depend on the size of the sketch and on the constraint
/// weights — `Fixed` carries ×50, `Parallel` scales with length, tangencies sit near 1 — so without
/// normalisation the rank drifted, reporting false redundancy or under-constraint on large and on tiny parts.
/// Null rows, left by a degenerate constraint, are left alone: they are genuinely dependent.
fn normalize_rows(jac: &mut [Vec<f64>]) {
    for row in jac.iter_mut() {
        let n = row.iter().map(|v| v * v).sum::<f64>().sqrt();
        if n > 1e-12 {
            for v in row.iter_mut() {
                *v /= n;
            }
        }
    }
}

/// Which points can still move (`true`), used to highlight the under-constrained ones.
pub fn free_points(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> Vec<bool> {
    // part by part, as `dof`: a point of a part with no constraint is free
    let mut out = vec![true; points.len()];
    for part in parts(points, radii, constraints).iter().filter(|p| !p.constraints.is_empty()) {
        let own = part.own(points, radii, constraints);
        for (&i, f) in part.points.iter().zip(free_points_whole(&own.points, &own.radii, &own.constraints)) {
            out[i] = f;
        }
    }
    out
}

/// THE DEGREES OF FREEDOM `constraints[k]` TAKES: those of its part without it less those with it - 0 for a
/// constraint that follows from the others. Counted in its own part alone: no other part changes its rank with it.
/// A constraint laid while drawing in a sketch of 10 000 lines counted the whole sketch twice.
pub fn freedom_taken(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint], k: usize) -> i32 {
    let Some(PartOf { part, at }) = part_of(points, radii, constraints, k) else { return 0 };
    let own = part.own(points, radii, constraints);
    taken(&own, at)
}

/// THE SAME, judged where the constraints hold: on a copy of the part solved with them all, to the thresholds of the
/// whole sketch, as a solve of the sketch solves that part.
pub fn freedom_taken_where_solved(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint], k: usize) -> i32 {
    let Some(PartOf { part, at }) = part_of(points, radii, constraints, k) else { return 0 };
    let mut own = part.own(points, radii, constraints);
    let how = Solve { scale: Scale::of(points), algebra: Algebra::BySize };
    guarded(&mut own.points, &mut own.radii, |points, radii| solve_full_iter_inner(points, radii, &own.constraints, None, 120, how));
    taken(&own, at)
}

/// The part that holds a constraint, and the place of the constraint among the constraints of the part.
struct PartOf {
    part: Part,
    at: usize,
}

fn part_of(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint], k: usize) -> Option<PartOf> {
    parts(points, radii, constraints).into_iter().find_map(|part| {
        let at = part.constraints.iter().position(|&ci| ci == k)?;
        Some(PartOf { part, at })
    })
}

/// The degrees of freedom of a part without its constraint `at` less those with it.
fn taken(own: &Own, at: usize) -> i32 {
    let without: Vec<Constraint> = own.constraints.iter().enumerate().filter(|(t, _)| *t != at).map(|(_, c)| c.clone()).collect();
    dof_whole(&own.points, &own.radii, &without).0 - dof_whole(&own.points, &own.radii, &own.constraints).0
}

/// THE REDUNDANT CONSTRAINTS among the first `own` of `constraints` (the rest are those of the entities themselves, such
/// as the ends of an arc on its circle): each whose removal frees no degree of freedom. Counted part by part, and only in
/// a part with an excess: a removal changes the rank of its own part alone. Counted whole, every constraint took two
/// counts of the whole sketch - 49 s on 10 000 lines for one count.
pub fn redundant(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint], own: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for part in parts(points, radii, constraints) {
        let mine = part.own(points, radii, constraints);
        let (free_all, excess) = dof_whole(&mine.points, &mine.radii, &mine.constraints);
        if excess <= 0 {
            continue;
        }
        for (k, &ci) in part.constraints.iter().enumerate().filter(|(_, ci)| **ci < own) {
            let without: Vec<Constraint> = mine.constraints.iter().enumerate().filter(|(t, _)| *t != k).map(|(_, c)| c.clone()).collect();
            if dof_whole(&mine.points, &mine.radii, &without).0 == free_all {
                out.push(ci);
            }
        }
    }
    out.sort_unstable();
    out
}

/// The free points of the whole sketch from one Jacobian (`free_points`); the reference of the parts.
pub fn free_points_whole(points: &[SketchPoint], radii: &[RadiusVar], constraints: &[Constraint]) -> Vec<bool> {
    let n = points.len();
    if n == 0 {
        return Vec::new();
    }
    let idx: HashMap<Id, usize> = points.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
    let ridx: HashMap<Id, usize> = radii.iter().enumerate().map(|(j, rv)| (rv.center, n * 2 + j)).collect();
    let has = |id: Id| idx.contains_key(&id);
    let is_center = |id: Id| ridx.contains_key(&id);
    let cons: Vec<Constraint> = constraints.iter().filter(|&c| cons_ok(c, &has, &is_center)).cloned().collect();
    let nv = n * 2 + radii.len();
    let anchor: HashMap<Id, (f64, f64)> = cons
        .iter()
        .filter_map(|c| match *c {
            Constraint::Fixed { p } => idx.get(&p).map(|&i| (p, (points[i].x, points[i].y))),
            _ => None,
        })
        .collect();
    let mut x: Vec<f64> = points.iter().flat_map(|p| [p.x, p.y]).collect();
    x.extend(radii.iter().map(|rv| rv.value));
    let r0 = residuals_of(&x, &x, &idx, &ridx, &anchor, &cons); // the side of an axis dimension is read off the current configuration
    let m = r0.len();
    if m == 0 {
        return vec![true; n]; // no constraints at all, so everything is free
    }
    let h = 1e-6;
    let mut jac = vec![vec![0.0_f64; nv]; m];
    for k in 0..nv {
        let mut xp = x.clone();
        xp[k] += h;
        let rp = residuals_of(&xp, &x, &idx, &ridx, &anchor, &cons);
        for i in 0..m {
            jac[i][k] = (rp[i] - r0[i]) / h;
        }
    }
    normalize_rows(&mut jac); // the rank must not depend on the scale of the sketch or on constraint weights
    let piv: std::collections::HashSet<usize> = pivot_columns(&mut jac, nv).into_iter().collect();
    (0..n).map(|i| !piv.contains(&(2 * i)) || !piv.contains(&(2 * i + 1))).collect()
}

/// Pivot columns of a rows×cols matrix, by Gaussian elimination with partial pivoting. The number of pivot
/// columns is the rank.
// THE INDEX IS THE MEANING: the same elimination as above, over the columns of the Jacobian.
#[allow(clippy::needless_range_loop)]
fn pivot_columns(a: &mut [Vec<f64>], cols: usize) -> Vec<usize> {
    let rows = a.len();
    let mut pivots = Vec::new();
    let mut row = 0usize;
    for col in 0..cols {
        if row >= rows {
            break;
        }
        // pivot element in the current column
        let mut piv = row;
        for r in row + 1..rows {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        if a[piv][col].abs() < 1e-7 {
            continue;
        }
        a.swap(row, piv);
        let d = a[row][col];
        for r in 0..rows {
            if r == row {
                continue;
            }
            let f = a[r][col] / d;
            if f != 0.0 {
                for c in col..cols {
                    a[r][c] -= f * a[row][c];
                }
            }
        }
        pivots.push(col);
        row += 1;
    }
    pivots
}
