//! EVERY TOOL OF A SKETCH KEEPS ITS TIME ON THE SKETCH OF THE REPORT: 973 lines and four patterns tied by 1 062 points on
//! lines into one part, 26 335 loops. Each tool through the door of the project the window calls, and after it the checks
//! the window counts again on a change (degrees of freedom, free points, conflicts, redundant) - each within its time.
//!
//! Reported behaviour: in that sketch every point of a line took some 20 s. The matrix of the tools on big sketches had no
//! sketch of one big part tied by points on lines, and the line was the only tool tried on it. The sketch has no corner
//! where two lines end at one point: the fillets and chamfers of corners, the offset and the extension find nothing to
//! work on in it and are timed idle.
#[path = "../../qymcad-core/tests/sketch_tools/mod.rs"]
mod sketch_tools;

use qymcad_core::model::Project;
use sketch_tools::tools;
use std::time::{Duration, Instant};

/// The time of `work`.
fn timed(work: impl FnOnce()) -> Duration {
    let started = Instant::now();
    work();
    started.elapsed()
}

/// The checks the window counts again when the sketch changed (`sketch_diag`).
fn checks(p: &Project) {
    let _ = p.sketch_checks(0);
    let _ = p.sketch_conflicts(0);
}

#[test]
fn every_tool_keeps_its_time_on_the_sketch_of_the_report() {
    let path = format!("{}/../../samples/pref_sketch.qcad", env!("CARGO_MANIFEST_DIR"));
    if !std::path::Path::new(&path).exists() {
        eprintln!("PASSED OVER: the private sample pref_sketch.qcad is not in this tree - the check on it runs only where the samples are");
        return;
    }
    let project = qymcad_io::load_project(&path).expect("the sample opens");
    let mut failures = Vec::new();
    // measured in a release build: a tool 0.03 ms - 0.34 s (50 lines turned the slowest); the checks after it 148 - 233 ms
    // each counted apart, 57 - 98 ms counted together with the dependencies made for the rows that need them
    let (tool_budget, check_budget) = (Duration::from_millis(600), Duration::from_millis(120));
    for tool in tools() {
        let mut p = project.clone();
        let took = timed(|| (tool.work)(&mut p));
        let counted = timed(|| checks(&p));
        eprintln!("the sketch of the report - {:<40} {took:>12.3?}, the checks after it {counted:>12.3?}", tool.name);
        if took > tool_budget || counted > check_budget {
            failures.push(format!("{}: {took:.3?} (budget {tool_budget:?}), the checks after it {counted:.3?} (budget {check_budget:?})", tool.name));
        }
    }
    assert!(failures.is_empty(), "tools slow on the sketch of the report:\n{}", failures.join("\n"));
}
