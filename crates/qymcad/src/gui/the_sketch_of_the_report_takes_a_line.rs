//! THE SKETCH OF THE REPORT TAKES A LINE: the document of the report opened through File, the part and then its sketch
//! entered by a double click in the tree, the Line tool taken and two points clicked - each step in the time of a few
//! frames. The sketch: 973 lines and four patterns tied by 1 062 points on lines into one part, 26 335 loops.
//!
//! Reported behaviour: in that sketch every point of a line took some 20 s, and nothing could be drawn. Each change
//! counted the redundant constraints one by one over the whole part (9 s), and each frame with the sketch selected in
//! the 3D view looked for every loop along the list of the sketch's loops (4.3 s).
#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use crate::gui::import_door::tests::running;
    use std::time::{Duration, Instant};

    /// The time of `work`.
    fn timed(work: impl FnOnce()) -> Duration {
        let started = Instant::now();
        work();
        started.elapsed()
    }

    /// A step of the hand and its time.
    struct Step {
        what: &'static str,
        took: Duration,
        kind: Kind,
    }

    /// What a step is: an action of the hand, several frames, or one frame of the window, or the way to the sketch -
    /// timed and told, not held to a time: entering the part rebuilds its bodies in the background, 0.36 s alone and
    /// 8.7 s beside a thousand other checks
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Action,
        Frame,
        Way,
    }

    #[test]
    fn the_sketch_of_the_report_takes_a_line() {
        let path = format!("{}/../../samples/pref_sketch.qcad", env!("CARGO_MANIFEST_DIR"));
        if !std::path::Path::new(&path).exists() {
            eprintln!("PASSED OVER: the private sample pref_sketch.qcad is not in this tree - the check on it runs only where the samples are");
            return;
        }
        let (mut app, _ctx) = running();
        // File, Open: the chooser answers with the document, and the load runs in the background as in the window
        let (tx, rx) = std::sync::mpsc::channel();
        app.arm_file_ask(rx, |app, p| crate::gui::io_jobs::spawn_project_load(&mut app.regen, p.to_string_lossy().into_owned()));
        tx.send(Some(std::path::PathBuf::from(&path))).expect("the chooser's channel is open");
        let mut hand = Hand::new(&mut app);
        let opening = Instant::now();
        hand.frame(Vec::new());
        while (hand.app.regen.busy.is_some() || hand.app.project.sketches.is_empty()) && opening.elapsed() < Duration::from_secs(300) {
            std::thread::sleep(Duration::from_millis(20));
            hand.frame(Vec::new());
        }
        let name = hand.app.project.sketches.first().map(|s| s.name.clone()).expect("the document holds its sketch");
        let lines = hand.app.project.sketches[0].entities.len();
        let mut steps: Vec<Step> = Vec::new();
        let took = timed(|| assert!(hand.double_click_word("Part 1"), "no row of the part in the tree"));
        steps.push(Step { what: "the part entered", took, kind: Kind::Way });
        let took = timed(|| assert!(hand.double_click_word(&name), "no row {name:?} in the tree"));
        steps.push(Step { what: "the sketch entered", took, kind: Kind::Action });
        let took = timed(|| {
            hand.frame(Vec::new());
        });
        steps.push(Step { what: "a frame of the sketch", took, kind: Kind::Frame });
        let took = timed(|| {
            hand.sk_tool(1);
        });
        steps.push(Step { what: "the Line tool taken", took, kind: Kind::Action });
        let took = timed(|| {
            hand.click2d(-500.0, -500.0);
        });
        steps.push(Step { what: "the first point", took, kind: Kind::Action });
        let took = timed(|| {
            hand.click2d(-400.0, -450.0);
        });
        steps.push(Step { what: "the second point", took, kind: Kind::Action });
        let lines_drawn = hand.app.project.sketches[0].entities.len();
        // the tool put down with Esc and the sketch left with Ctrl+Enter: the view is back in 3D with the sketch selected
        let took = timed(|| {
            hand.key(egui::Key::Escape).key(egui::Key::Escape).ctrl(egui::Key::Enter);
        });
        steps.push(Step { what: "the sketch left", took, kind: Kind::Action });
        assert!(hand.app.sketch_ses.editing.is_none(), "Ctrl+Enter did not leave the sketch");
        assert!(hand.app.viewing.mode_3d && matches!(hand.app.chosen.sel, super::super::Sel::Sketch(0)), "the sketch left is not selected in the 3D view");
        let took = timed(|| {
            hand.frame(Vec::new());
        });
        steps.push(Step { what: "a frame of the 3D view", took, kind: Kind::Frame });
        let said: Vec<String> = steps.iter().map(|s| format!("{} {:?}", s.what, s.took)).collect();
        eprintln!("the sketch of the report: {}", said.join(", "));
        assert_eq!(lines_drawn, lines + 1, "the two clicks drew no line");
        // measured in a release build: an action 0.18 - 0.39 s, some 20 s a point before; a frame 30 ms, 4.3 s before in
        // the 3D view with the sketch selected. A test build is several times slower
        let budget = |k: Kind| match (k, cfg!(debug_assertions)) {
            (Kind::Way, _) => Duration::MAX,
            (Kind::Action, true) => Duration::from_secs(5),
            (Kind::Action, false) => Duration::from_secs(1),
            (Kind::Frame, true) => Duration::from_secs(1),
            (Kind::Frame, false) => Duration::from_millis(200),
        };
        assert!(steps.iter().all(|s| s.took <= budget(s.kind)), "steps over their budget in the sketch of the report: {}", said.join(", "));
    }
}
