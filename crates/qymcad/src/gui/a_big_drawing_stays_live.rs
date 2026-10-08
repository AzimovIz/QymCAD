//! A BIG DRAWING STAYS LIVE, through the window: a DXF of thousands of segments, written by the check itself, comes in
//! through "Import", is laid on a plane, and the sketch it makes is worked by hand - the pointer over it, a corner
//! dragged - each frame timed.
//!
//! Reported behaviour (#95): a sketch of a few hundred lines hangs the program.
#[cfg(test)]
pub(crate) mod tests {
    use super::super::hand::Hand;
    use crate::gui::import_door::tests::{answer, running};
    use qymcad_ui_state::Want;
    use std::time::{Duration, Instant};

    /// A DXF of `n` separate rectangles of four LINE entities each, 10 by 6 mm, on a grid 20 mm apart.
    pub(crate) fn rectangles_dxf(n: usize) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(format!("{}/../../target/a-big-drawing", env!("CARGO_MANIFEST_DIR")));
        std::fs::create_dir_all(&dir).expect("a folder for the check");
        let side = (n as f64).sqrt().ceil() as usize;
        let mut dxf = String::from("0\nSECTION\n2\nENTITIES\n");
        for k in 0..n {
            let (x, y) = ((k % side) as f64 * 20.0, (k / side) as f64 * 20.0);
            let c = [(x, y), (x + 10.0, y), (x + 10.0, y + 6.0), (x, y + 6.0)];
            for i in 0..4 {
                let (a, b) = (c[i], c[(i + 1) % 4]);
                dxf.push_str(&format!("0\nLINE\n8\n0\n10\n{}\n20\n{}\n30\n0.0\n11\n{}\n21\n{}\n31\n0.0\n", a.0, a.1, b.0, b.1));
            }
        }
        dxf.push_str("0\nENDSEC\n0\nEOF\n");
        let p = dir.join(format!("rectangles-{n}.dxf"));
        std::fs::write(&p, dxf).expect("written");
        p
    }

    /// The time of `work`.
    fn timed(work: impl FnOnce()) -> Duration {
        let started = Instant::now();
        work();
        started.elapsed()
    }

    #[test]
    fn a_big_drawing_is_worked_by_hand() {
        // A RELEASE BUILD TAKES THE DRAWING OF THE BAR, 70 000 segments, and holds a frame to its time; a test build, ten
        // times slower, takes 10 000 and holds what is drawn
        let release = !cfg!(debug_assertions);
        let n = if release { 17_500 } else { 2_500 };
        let path = rectangles_dxf(n);
        let (mut app, ctx) = running();
        let t_import = timed(|| answer(&mut app, &ctx, Want::Anything, &path.to_string_lossy()));
        let mut hand = Hand::new(&mut app);
        let t_place = timed(|| {
            hand.click([5.0, 5.0, 0.0]);
        });
        let si = hand.app.project.sketches.iter().position(|s| s.entities.len() >= 4 * n).expect("the drawing came in as a sketch");
        let t_frame = timed(|| {
            hand.frame(Vec::new());
        });
        // the whole drawing in sight, 10 000 points: the plain ones are not drawn (a ring or so of the window's own marks
        // is of that size), nor any number
        let plain = |hand: &Hand| hand.rings_drawn(3.5).len();
        assert!(plain(&hand) < 10, "{} points of 10 000 in sight are drawn one by one", plain(&hand));
        assert!(!hand.shows("10000"), "the numbers of 10 000 points in sight are written");
        let t_hover = timed(|| {
            hand.hover2d(10.0, 6.0);
        });
        let t_drag = timed(|| {
            hand.drag2d((10.0, 6.0), (13.0, 9.0));
        });
        // two frames with the pointer over the drawing. Measured on 70 000 segments: 0.4 s before the pick and the snap
        // looked up their points from a table, 0.27 s before the keys of the status were mixed quickly, 0.21 s before
        // the panel and the glyphs stopped gathering the points of every entity, 0.12 s after
        if release {
            assert!(t_hover < Duration::from_millis(200), "two frames with the pointer over 70 000 segments took {t_hover:?}, budget 200 ms in a release build");
        }
        // brought near a corner, a few hundred points in sight: drawn and numbered again
        hand.look2d((30.0, 30.0)).frame(Vec::new());
        let near = plain(&hand);
        assert!(near > 0 && near <= qymcad_render::DOTTED_POINTS, "near a corner {near} plain points are drawn");
        assert!(hand.shows("1"), "near a corner the points are not numbered");
        // the pointer inside a rectangle, no corner within reach: the snap looks for what runs near it, not for every
        // crossing of every pair of segments (5e7 pairs here, 2.45e9 on 70 000 segments - 15 s a frame)
        let t_away = timed(|| {
            hand.hover2d(35.0, 33.0);
        });
        assert!(t_away < Duration::from_secs(1), "the pointer away from the corners took {t_away:?} for its two frames, budget 1 s in a test build");
        eprintln!("import {t_import:?}, place {t_place:?}, frame {t_frame:?}, hover {t_hover:?}, drag {t_drag:?}, away {t_away:?}; {} entities", hand.app.project.sketches[si].entities.len());
    }
}
