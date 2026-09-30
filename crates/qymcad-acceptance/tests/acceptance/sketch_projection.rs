//! WHAT A SKETCH TAKES FROM THE BODY UNDER IT, and what it draws that is not part of the shape: an edge or the whole
//! outline of a face brought in as driven geometry, and construction lines that stay outside the profile.
use qymcad::{Key, PointerButton, Session};
use qymcad_acceptance::build;
use qymcad_acceptance::probe;

/// Take the tool whose hint is `hint`.
fn take(s: &mut Session, hint: &str) {
    let hint = s.word(hint);
    s.press_hint(&hint);
}

/// Press the word of the bar of options `key` names.
fn bar_word(s: &mut Session, key: &str) {
    let word = s.word(key);
    s.press_word_near(&word, qymcad::pos2(640.0, 24.0));
}

/// How many lines the sketch holds, and how far it reaches.
fn lines_and_box(s: &mut Session) -> (usize, [f64; 2], [f64; 2]) {
    let sk = s.document().sketches.last().cloned().unwrap_or_else(|| panic!("the document holds no sketch"));
    (sk.lines, sk.min, sk.max)
}

/// The 40 x 30 x 10 block of the first part with a sketch open on its top face.
fn a_sketch_on_the_top_of_a_block() -> Session {
    let mut s = Session::start();
    build::block(&mut s);
    build::sketch_on_face(&mut s, [20.0, 15.0, 10.0]);
    s
}

probe! {
    /// AN EDGE OF THE BODY IS BROUGHT INTO THE SKETCH by a click, and the program says it is driven.
    fn an_edge_of_the_body_is_projected_into_the_sketch() {
        let mut s = a_sketch_on_the_top_of_a_block();
        assert!(lines_and_box(&mut s).0 == 0, "the new sketch is not empty: {:?}", lines_and_box(&mut s));
        take(&mut s, "tb-project-body-hint");
        s.click_on_sketch(20.0, 0.0); // the front edge of the top face, under the sheet
        assert!(s.status() == s.word("sk-projected-hint"), "nothing says the geometry is driven: the status line says {:?}", s.status());
        let (lines, min, max) = lines_and_box(&mut s);
        assert!(lines == 1, "the edge did not come into the sketch: it holds {lines} line(s)");
        let (w, h) = (max[0] - min[0], max[1] - min[1]);
        assert!((w.max(h) - 40.0).abs() < 1e-3 && w.min(h) < 1e-3, "what came in is not the 40 long edge of the block: it is {w} by {h}");
    }
}

probe! {
    /// A PROJECTED EDGE IS DRIVEN: it does not follow the mouse, as the program says.
    fn a_projected_edge_does_not_follow_the_mouse() {
        let mut s = a_sketch_on_the_top_of_a_block();
        take(&mut s, "tb-project-body-hint");
        s.click_on_sketch(20.0, 0.0); // the front edge of the top face, under the sheet
        let before = s.document().sketches.last().cloned().expect("the sketch").places;
        let arrow = s.word("tb-select-hint");
        s.press_hint(&arrow);
        let (from, to) = (before[0], [before[0][0] + 5.0, before[0][1] + 5.0]);
        s.drag_on_sketch((from[0], from[1]), (to[0], to[1]));
        let after = s.document().sketches.last().cloned().expect("the sketch").places;
        assert!(after == before, "the projected edge followed the mouse: {before:?} became {after:?}");
    }
}

probe! {
    /// THE WHOLE OUTLINE OF THE FACE UNDER THE SKETCH comes in at once when the bar is set to it.
    fn the_outline_of_the_face_under_the_sketch_is_projected() {
        let mut s = a_sketch_on_the_top_of_a_block();
        take(&mut s, "tb-project-body-hint");
        bar_word(&mut s, "opt-face-outline");
        s.click_on_sketch(20.0, 15.0);
        let (lines, min, max) = lines_and_box(&mut s);
        assert!(lines == 4, "the outline of the top face is four lines, and {lines} came in");
        let (w, h) = (max[0] - min[0], max[1] - min[1]);
        assert!((w.max(h) - 40.0).abs() < 1e-3 && (w.min(h) - 30.0).abs() < 1e-3, "the outline is not the 40 by 30 top of the block: it is {w} by {h}");
    }
}

probe! {
    /// THE OUTLINE OF A FACE IS REFUSED IN WORDS for a sketch that sits on a plane and not on a face, and nothing is
    /// drawn.
    fn the_outline_of_a_face_is_refused_for_a_sketch_on_a_plane() {
        let mut s = Session::start();
        build::into_the_first_part(&mut s);
        let xy = s.word("plane-xy-table");
        s.press_word(&xy);
        take(&mut s, "tb-project-body-hint");
        bar_word(&mut s, "opt-face-outline");
        s.click_on_sketch(10.0, 10.0);
        assert!(s.status() == s.word("sk-face-outline-only"), "nothing says the outline needs a sketch on a face: the status line says {:?}", s.status());
        assert!(lines_and_box(&mut s).0 == 0, "something was drawn anyway: {:?}", lines_and_box(&mut s));
    }
}

probe! {
    /// A PROJECTED EDGE FOLLOWS THE PART: the block is made wider by its own sketch, and the line that was taken from
    /// its edge grows with it.
    fn a_projected_edge_follows_the_part() {
        let mut s = a_sketch_on_the_top_of_a_block();
        take(&mut s, "tb-project-body-hint");
        s.click_on_sketch(20.0, 0.0); // the front edge of the top face, under the sheet
        let finish = s.word("wb-finish");
        s.press_word(&finish);
        // the rectangle the block was made of, made 60 wide by dragging its corner
        let first = s.document().sketches[0].name.clone();
        let row = s.find(&first, qymcad::pos2(0.0, 300.0)).unwrap_or_else(|| panic!("the first sketch is not in the tree; on screen: {:?}", s.words()));
        s.double_click(row.center());
        s.drag_on_sketch((40.0, 30.0), (60.0, 30.0));
        let finish = s.word("wb-finish");
        s.press_word(&finish);
        let (lines, min, max) = lines_and_box(&mut s);
        let (w, h) = (max[0] - min[0], max[1] - min[1]);
        assert!(lines == 1, "the projected line is gone: the sketch holds {lines} line(s)");
        assert!((w.max(h) - 60.0).abs() < 1e-3, "the projected line did not follow the part from 40 to 60: it is {w} by {h}");
    }
}

probe! {
    /// CONSTRUCTION GEOMETRY STAYS OUT OF THE PROFILE: a rectangle drawn as construction is in the sketch and makes no
    /// body.
    fn construction_geometry_makes_no_body() {
        let mut s = build::empty_sketch();
        let switch = s.word("opt-construction-short");
        s.toggle(&switch);
        build::draw(&mut s, "tb-rect-hint", &[(0.0, 0.0), (40.0, 30.0)]);
        assert!(lines_and_box(&mut s).0 == 4, "the rectangle was not drawn: {:?}", lines_and_box(&mut s));
        let finish = s.word("wb-finish");
        s.press_word(&finish);
        let extrude = s.word("tb-extrude-hint");
        s.press_hint(&extrude);
        s.key(Key::Enter);
        assert!(s.document().bodies.is_empty(), "construction geometry was extruded into a body: {:?}", s.document().bodies);
    }
}

probe! {
    /// GEOMETRY ALREADY DRAWN IS TURNED INTO CONSTRUCTION AND BACK, and the profile goes and comes back with it.
    fn geometry_is_turned_into_construction_and_back() {
        let mut s = build::empty_sketch();
        build::draw(&mut s, "tb-rect-hint", &[(0.0, 0.0), (40.0, 30.0)]);
        for (x, y) in [(20.0, 0.0), (40.0, 15.0), (20.0, 30.0), (0.0, 15.0)] {
            build::pick(&mut s, x, y, (x, y) != (20.0, 0.0));
        }
        let at = s.on_sketch(20.0, 0.0);
        s.click_with(at, PointerButton::Secondary, qymcad::Modifiers::default());
        let toggle = s.word("sk-construction-toggle");
        s.press_word_near(&toggle, at);
        let finish = s.word("wb-finish");
        s.press_word(&finish);
        let extrude = s.word("tb-extrude-hint");
        s.press_hint(&extrude);
        s.key(Key::Enter);
        assert!(s.document().bodies.is_empty(), "the rectangle turned into construction still made a body: {:?}", s.document().bodies);
    }
}
