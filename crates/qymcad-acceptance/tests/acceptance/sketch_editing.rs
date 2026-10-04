//! EDITING WHAT IS ALREADY DRAWN: trimming, extending, breaking, rounding and cutting corners, offsetting,
//! mirroring, deleting, moving, copying, turning and laying out in patterns.
//!
//! Each check draws what it needs with the auto constraints turned off, so that the geometry stays where it was put
//! and what the tool did to it is the only change. What came of it is read off the sheet: how many lines and arcs
//! there are, where their ends stand, and how far the drawing reaches.
use qymcad::{Key, Kind, Modifiers, PointerButton, Session, Widget};
use qymcad_acceptance::build::{draw, empty_sketch, line, pick};
use qymcad_acceptance::probe;

/// Take the tool whose hint is `hint` and answer what the program said about it.
fn take(s: &mut Session, hint: &str) -> String {
    let hint = s.word(hint);
    s.press_hint(&hint);
    s.status()
}

/// Type `text` into a field of the bar found by sight rather than by a caption: the fields of a pattern stand in a
/// row and only the first of them is named.
fn fill_widget(s: &mut Session, w: &Widget, text: &str) {
    s.click(w.rect.center()).chord(Modifiers::COMMAND, Key::A).type_text(text);
}

/// The fields of the bar of options, left to right.
fn bar_fields(s: &mut Session) -> Vec<Widget> {
    let mut fields: Vec<Widget> = s.widgets().into_iter().filter(|w| w.kind == Kind::TextField && w.rect.top() < 120.0).collect();
    fields.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
    fields
}

/// THE LITTLE BOX THAT POPPED UP BY THE GESTURE: the field nearest to `at` - the radius at a corner, the angle at the
/// centre of a turn - told apart from the fields of the bar, which stand far away at the top.
fn field_near(s: &mut Session, at: qymcad::Pos2) -> Widget {
    let fields: Vec<Widget> = s.widgets().into_iter().filter(|w| w.kind == Kind::TextField && w.rect.top() > 120.0).collect();
    fields
        .iter()
        .min_by(|a, b| a.rect.center().distance(at).total_cmp(&b.rect.center().distance(at)))
        .cloned()
        .unwrap_or_else(|| panic!("no little box popped up near {at:?}; the fields on screen are {fields:?}"))
}

/// How far the drawing reaches: the corners of the box around it.
fn box_of(s: &mut Session) -> ([f64; 2], [f64; 2]) {
    let sk = s.document().sketches[0].clone();
    (sk.min, sk.max)
}

/// Whether the box around the drawing runs from `min` to `max`, to a thousandth of a millimetre.
fn box_is(s: &mut Session, min: [f64; 2], max: [f64; 2]) -> bool {
    let (a, b) = box_of(s);
    a.iter().zip(min).all(|(x, y)| (x - y).abs() < 1e-3) && b.iter().zip(max).all(|(x, y)| (x - y).abs() < 1e-3)
}

/// A PICK WITH SHIFT: it does not take the corner away and make a new one, it joins the one in hand.
fn pick_shift(s: &mut Session, x: f64, y: f64) {
    let at = s.on_sketch(x, y);
    s.click_with(at, PointerButton::Primary, Modifiers::SHIFT);
}

/// FOUR LINES STANDING ON ONE POINT, each a spoke of it, and the place to click each of them: the middle of the
/// spoke. They are drawn a long way out, because the field of a corner stands where the corner was picked, and a pick
/// with Shift that lands on its buttons answers the corner instead of joining the set.
fn cross_of_four_lines(s: &mut Session) -> [(f64, f64); 4] {
    [((0.0, 0.0), (3000.0, 0.0)), ((0.0, 0.0), (0.0, 3000.0)), ((0.0, 0.0), (-3000.0, 0.0)), ((0.0, 0.0), (0.0, -3000.0))]
        .into_iter()
        .map(|(a, b)| {
            line(s, a, b);
            ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or_else(|_| unreachable!("four spokes make four places to click them"))
}

/// How many lines, arcs and circles the sketch holds.
fn counts(s: &mut Session) -> (usize, usize, usize) {
    let sk = s.document().sketches[0].clone();
    (sk.lines, sk.arcs, sk.circles)
}

/// Whether any end of the geometry stands at `p`.
fn stands_at(s: &mut Session, p: (f64, f64)) -> bool {
    s.document().sketches[0].places.iter().any(|q| (q[0] - p.0).hypot(q[1] - p.1) < 1e-6)
}

probe! {
    /// TRIM BY A CLICK: the piece of a line beyond the crossing goes, and the rest of the drawing stays.
    fn trim_by_a_click_takes_the_piece_beyond_the_crossing() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (40.0, 0.0));
        line(&mut s, (20.0, -10.0), (20.0, 10.0));
        take(&mut s, "tb-trim-hint");
        s.click_on_sketch(35.0, 0.0);
        let (min, max) = box_of(&mut s);
        assert!((max[0] - 20.0).abs() < 1e-6, "the piece beyond the crossing is still there: the drawing reaches {}", max[0]);
        assert!((min[0]).abs() < 1e-3 && (min[1] + 10.0).abs() < 1e-3, "trimming moved the rest of the drawing: it starts at {min:?}");
        assert!(counts(&mut s).0 == 2, "trimming took a whole line away: {:?}", counts(&mut s));
    }
}

probe! {
    /// TRIM BY A DRAG: everything the cursor passes through is trimmed, in one gesture.
    fn trim_by_a_drag_takes_everything_it_passes_through() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (60.0, 0.0));
        for x in [10.0, 30.0, 50.0] {
            line(&mut s, (x, -10.0), (x, 10.0));
        }
        take(&mut s, "tb-trim-hint");
        s.drag_on_sketch((5.0, -5.0), (55.0, -5.0));
        let (min, _) = box_of(&mut s);
        assert!((min[1]).abs() < 1e-6, "the pieces the drag passed through are still there: the drawing reaches down to {}", min[1]);
    }
}

probe! {
    /// MOVING THE SHEET WITH THE MIDDLE BUTTON CUTS NOTHING, the trim in hand: only the left button trims by a drag.
    /// Reported behaviour: the sheet moved with the middle button to bring a point into view cut the line in two.
    fn moving_the_sheet_with_the_trim_in_hand_cuts_nothing() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (60.0, 0.0));
        for x in [10.0, 30.0, 50.0] {
            line(&mut s, (x, -10.0), (x, 10.0));
        }
        take(&mut s, "tb-trim-hint");
        // grabbed ON the upright line at 30, as the sheet is grabbed wherever the pointer happens to stand: the sheet
        // follows the pointer, so the point grabbed stays under it all the way
        let a = s.on_sketch(30.0, -5.0);
        let b = s.seen_on_sketch(40.0, -5.0).unwrap_or_else(|| panic!("the end of the move is out of view"));
        s.drag(a, b, qymcad::PointerButton::Middle, qymcad::Modifiers::NONE);
        s.key(qymcad::Key::Escape); // the trim put down, so that what lies at a point is read, not trimmed
        let under = s.sketch_under(30.0, -8.0);
        assert!(matches!(under, Some(qymcad::SketchPick::Line { .. })), "moving the sheet with the middle button cut the upright line it was grabbed on: under (30, -8) lies {under:?}");
    }
}

probe! {
    /// EXTEND: the end of a line is stretched to the line it stops short of.
    fn extend_stretches_a_line_to_what_it_stops_short_of() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (20.0, 0.0));
        line(&mut s, (30.0, -10.0), (30.0, 10.0));
        take(&mut s, "tb-extend-hint");
        s.click_on_sketch(18.0, 0.0);
        assert!(stands_at(&mut s, (30.0, 0.0)), "the line was not stretched to the crossing at (30, 0): the ends stand at {:?}", s.document().sketches[0].places);
    }
}

probe! {
    /// BREAK: one line becomes two, and the drawing keeps its shape.
    fn break_splits_a_line_in_two() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (40.0, 0.0));
        let before = box_of(&mut s);
        take(&mut s, "tb-break-hint");
        s.click_on_sketch(20.0, 0.0);
        assert!(counts(&mut s).0 == 2, "the line was not split in two: {:?}", counts(&mut s));
        assert!(box_of(&mut s) == before, "breaking the line moved it: {:?} became {:?}", before, box_of(&mut s));
    }
}

probe! {
    /// FILLET A CORNER: the corner of two lines becomes an arc of the radius that was typed.
    fn fillet_rounds_a_corner_with_the_radius_typed() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-line-hint", &[(30.0, 0.0), (0.0, 0.0), (0.0, 30.0)]);
        take(&mut s, "tb-fillet-sketch-hint");
        let corner = s.on_sketch(0.0, 0.0);
        s.click(corner);
        let field = field_near(&mut s, corner);
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).1 == 1, "the corner did not become an arc: {:?}", counts(&mut s));
        // the corner is cut back by the radius: the lines now start 5 away from where they met
        assert!(stands_at(&mut s, (5.0, 0.0)) && stands_at(&mut s, (0.0, 5.0)), "the arc of radius 5 does not meet the lines at (5, 0) and (0, 5): the ends stand at {:?}", s.document().sketches[0].places);
    }
}

probe! {
    /// CHAMFER A CORNER: the corner of two lines becomes a third line across it.
    fn chamfer_cuts_a_corner_with_the_leg_typed() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-line-hint", &[(30.0, 0.0), (0.0, 0.0), (0.0, 30.0)]);
        take(&mut s, "tb-chamfer-sketch-hint");
        let corner = s.on_sketch(0.0, 0.0);
        s.click(corner);
        let field = field_near(&mut s, corner);
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).0 == 3, "the corner did not become a line across: {:?}", counts(&mut s));
        assert!(stands_at(&mut s, (5.0, 0.0)) && stands_at(&mut s, (0.0, 5.0)), "the cut of 5 does not meet the lines at (5, 0) and (0, 5): the ends stand at {:?}", s.document().sketches[0].places);
    }
}

probe! {
    /// A CHAMFER POINTED AT BY ITS TWO LINES: the cursor finds the corner without finding the point. Reported: the
    /// corner of a long edge could only be taken by hitting the vertex itself within a few pixels.
    fn chamfer_of_two_lines_that_meet_is_taken_by_them_alone() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-line-hint", &[(60.0, 0.0), (0.0, 0.0), (0.0, 60.0)]);
        take(&mut s, "tb-chamfer-sketch-hint");
        s.click_on_sketch(30.0, 0.0); // the middle of the first line: not a point of the drawing at all
        let second = s.on_sketch(0.0, 30.0);
        s.click(second); // and the middle of the second: the corner is where the two meet
        let field = field_near(&mut s, second);
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).0 == 3, "the two lines did not cut the corner between them: {:?}", counts(&mut s));
        assert!(stands_at(&mut s, (5.0, 0.0)) && stands_at(&mut s, (0.0, 5.0)), "the cut of 5 does not meet the lines at (5, 0) and (0, 5): the ends stand at {:?}", s.document().sketches[0].places);
    }
}

probe! {
    /// A PICK THAT NAMES NO CORNER TAKES THE FIELD DOWN WITH IT. Reported: three lines were clicked in a row, the
    /// third not joined to the first two, and the preview of the corner of the first two stood on the sheet with its
    /// field - a corner that was no longer the pair on screen.
    fn a_pick_that_names_no_corner_leaves_no_field_behind() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-line-hint", &[(60.0, 0.0), (0.0, 0.0), (0.0, 60.0)]);
        draw(&mut s, "tb-line-hint", &[(140.0, 140.0), (120.0, 140.0), (120.0, 160.0)]); // an L far away
        take(&mut s, "tb-chamfer-sketch-hint");
        s.click_on_sketch(30.0, 0.0); // the middle of the first line
        let corner = s.on_sketch(0.0, 30.0);
        s.click(corner); // the middle of the second: the field of their corner opens
        let field = field_near(&mut s, corner);
        assert_eq!(field.kind, Kind::TextField, "the corner of the first two lines did not open its field");
        s.click_on_sketch(130.0, 140.0); // the middle of the third, which meets neither of them
        let left = s.widgets().into_iter().filter(|w| w.kind == Kind::TextField && w.rect.top() > 120.0).collect::<Vec<_>>();
        assert!(left.is_empty(), "the field of the corner before it stood open over a pair that was gone: {left:?}");
    }
}

probe! {
    /// LEAVING THE CORNER MODE LETS GO OF WHAT IT HAD CHOSEN. Reported: the lines it was offered went on standing lit
    /// with nothing in hand, and the next tool found a selection that was not its own.
    fn leaving_the_corner_mode_clears_what_it_had_chosen() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-line-hint", &[(60.0, 0.0), (0.0, 0.0), (0.0, 60.0)]);
        pick(&mut s, 30.0, 0.0, false);
        pick(&mut s, 0.0, 30.0, true); // the two lines of the corner, chosen before the tool
        take(&mut s, "tb-chamfer-sketch-hint"); // the corner of the two is offered at once, out of the selection
        let middle = s.canvas().center();
        let field = field_near(&mut s, middle); // the field of that corner is open
        assert_eq!(field.kind, Kind::TextField, "the corner of the two chosen lines did not open its field");
        take(&mut s, "tb-line-hint"); // another tool: leaving the chamfer takes its picks with it
        take(&mut s, "tb-chamfer-sketch-hint"); // and the chamfer asks for the corner rather than offering an old one
        let left: Vec<Widget> = s.widgets().into_iter().filter(|w| w.kind == Kind::TextField && w.rect.top() > 120.0).collect();
        assert!(left.is_empty(), "the mode was taken again and offered the corner of the two lines it had been given before, which the mode that ended had let go of: {left:?}");
    }
}

probe! {
    /// THE CHAMFER TAKEN WITH THE TWO LINES ALREADY CHOSEN: the corner is where they meet, and the tool is pressed
    /// afterwards. Reported: the selection stood lit and the tool ignored it, asking for the corner again.
    fn a_chamfer_of_two_lines_already_chosen_is_offered_the_corner_they_share() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-line-hint", &[(60.0, 0.0), (0.0, 0.0), (0.0, 60.0)]);
        pick(&mut s, 30.0, 0.0, false);
        pick(&mut s, 0.0, 30.0, true); // the two lines of the corner, chosen before the tool
        take(&mut s, "tb-chamfer-sketch-hint");
        let middle = s.canvas().center();
        let field = field_near(&mut s, middle);
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).0 == 3, "the corner of the two chosen lines did not become a line across: {:?}", counts(&mut s));
        assert!(stands_at(&mut s, (5.0, 0.0)) && stands_at(&mut s, (0.0, 5.0)), "the cut of 5 does not meet the lines at (5, 0) and (0, 5): the ends stand at {:?}", s.document().sketches[0].places);
    }
}

probe! {
    /// TWO LINES THAT SHARE NO CORNER: nothing is offered, and the tool waits for a corner of its own.
    fn two_lines_that_meet_nowhere_leave_the_selection_alone_to_wait() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (60.0, 0.0));
        line(&mut s, (30.0, -30.0), (30.0, 30.0)); // it crosses the first, but shares no point with it
        pick(&mut s, 10.0, 0.0, false);
        pick(&mut s, 30.0, 15.0, true);
        take(&mut s, "tb-chamfer-sketch-hint");
        let boxes: Vec<_> = s.widgets().into_iter().filter(|w| w.kind == Kind::TextField && w.rect.top() > 120.0).collect();
        assert!(boxes.is_empty(), "two lines with no corner in common opened the box anyway: {boxes:?}");
        assert!(counts(&mut s).0 == 2, "nothing was cut: {:?}", counts(&mut s));
    }
}

probe! {
    /// TWO LINES THAT SHARE NO CORNER: the first is let go, the second stands as the first of the next pair, and
    /// the search goes on with its neighbour.
    fn a_line_without_a_corner_becomes_the_first_of_the_next_one() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-line-hint", &[(60.0, 0.0), (0.0, 0.0), (0.0, 60.0)]);
        draw(&mut s, "tb-line-hint", &[(150.0, 0.0), (120.0, 0.0), (120.0, 30.0)]); // a second angle, far away
        take(&mut s, "tb-fillet-sketch-hint");
        s.click_on_sketch(30.0, 0.0); // the first arm of the near angle
        s.click_on_sketch(135.0, 0.0); // an arm of the far angle: no corner in common with the first
        let third = s.on_sketch(120.0, 15.0);
        s.click(third); // the other arm of the far angle, the neighbour of the one now chosen
        let field = field_near(&mut s, third);
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).1 == 1, "the search did not carry on from the line clicked last: {:?}", counts(&mut s));
        assert!(stands_at(&mut s, (125.0, 0.0)) && stands_at(&mut s, (120.0, 5.0)), "the arc of radius 5 does not meet the far lines at (125, 0) and (120, 5): the ends stand at {:?}", s.document().sketches[0].places);
    }
}

probe! {
    /// TWO SQUARES SHARING A SINGLE POINT: the corner there is taken between the two lines that meet at it, and the
    /// other square keeps its sharp corner. Reported: nothing could be done at such a point at all, and nothing said.
    fn a_point_of_four_lines_is_a_corner_of_the_two_chosen() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-rect-hint", &[(0.0, 0.0), (20.0, 20.0)]);
        draw(&mut s, "tb-rect-hint", &[(20.0, 20.0), (40.0, 40.0)]); // sharing the point (20, 20) and nothing else
        take(&mut s, "tb-chamfer-sketch-hint");
        s.click_on_sketch(20.0, 10.0); // the right side of the near square
        let second = s.on_sketch(10.0, 20.0); // and its top side: the corner they meet at
        s.click(second);
        let field = field_near(&mut s, second);
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).0 == 9, "the corner of the shared point was not cut: {:?}", counts(&mut s));
        assert!(stands_at(&mut s, (20.0, 15.0)) && stands_at(&mut s, (15.0, 20.0)), "the cut of 5 does not meet the sides of the near square at (20, 15) and (15, 20): the ends stand at {:?}", s.document().sketches[0].places);
        assert!(!stands_at(&mut s, (25.0, 20.0)) && !stands_at(&mut s, (20.0, 25.0)), "the far square lost its corner as well: four lines through one point made two cuts of one corner");
    }
}

probe! {
    /// TWO HALVES OF ONE STRAIGHT LINE: they share a point and make an angle of 180 degrees, which is no corner. The
    /// search goes on, and the neighbour of the second half is the corner that is offered.
    fn a_straight_joint_is_not_a_corner_and_the_search_carries_on() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (20.0, 0.0));
        line(&mut s, (20.0, 0.0), (40.0, 0.0)); // the same straight line in two pieces
        line(&mut s, (40.0, 0.0), (40.0, 30.0));
        take(&mut s, "tb-fillet-sketch-hint");
        s.click_on_sketch(10.0, 0.0); // the first half
        s.click_on_sketch(30.0, 0.0); // the second: 180 degrees is no corner, so it stands as the first of the next pair
        let third = s.on_sketch(40.0, 15.0);
        s.click(third); // its neighbour: here there IS a corner
        let field = field_near(&mut s, third);
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).1 == 1, "the joint of two halves of one straight line was rounded: {:?}", counts(&mut s));
        assert!(stands_at(&mut s, (35.0, 0.0)) && stands_at(&mut s, (40.0, 5.0)), "the arc of radius 5 does not meet the two lines at (35, 0) and (40, 5): the ends stand at {:?}", s.document().sketches[0].places);
    }
}

probe! {
    /// FILLET EVERY CORNER: all four corners of a rectangle become arcs at once.
    fn fillet_all_corners_rounds_the_whole_rectangle() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-rect-hint", &[(0.0, 0.0), (40.0, 30.0)]);
        take(&mut s, "tb-fillet-all-hint");
        s.click_on_sketch(20.0, 0.0); // the shape to round, as the bar asks
        let middle = s.canvas().center();
        let field = field_near(&mut s, middle);
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).1 == 4, "the four corners did not all become arcs: {:?}", counts(&mut s));
        assert!(box_is(&mut s, [0.0, 0.0], [40.0, 30.0]), "rounding the corners changed the size of the rectangle: {:?}", box_of(&mut s));
    }
}

probe! {
    /// OFFSET: the picked contour gets a copy of itself at the distance that was set.
    fn offset_makes_a_copy_of_the_contour_at_a_distance() {
        let mut s = empty_sketch();
        draw(&mut s, "tb-rect-hint", &[(0.0, 0.0), (40.0, 30.0)]);
        take(&mut s, "tb-offset-hint"); // taken first: the distance stands in its bar
        let distance = s.word("opt-distance");
        s.fill(&distance, "5");
        for (x, y) in [(20.0, 0.0), (40.0, 15.0), (20.0, 30.0), (0.0, 15.0)] {
            pick(&mut s, x, y, (x, y) != (20.0, 0.0));
        }
        assert!(counts(&mut s).0 == 8, "the contour was not copied: {:?}", counts(&mut s));
        assert!(box_is(&mut s, [-5.0, -5.0], [45.0, 35.0]) || box_is(&mut s, [0.0, 0.0], [40.0, 30.0]), "the copy does not stand 5 from the contour: {:?}", box_of(&mut s));
    }
}

probe! {
    /// MIRROR: what is picked is reflected about the axis pointed at.
    fn mirror_reflects_what_is_picked_about_an_axis() {
        let mut s = empty_sketch();
        line(&mut s, (10.0, 0.0), (30.0, 20.0));
        line(&mut s, (0.0, -20.0), (0.0, 20.0)); // the axis to reflect about
        pick(&mut s, 20.0, 10.0, false);
        take(&mut s, "tb-mirror-sketch-hint");
        pick(&mut s, 0.0, 10.0, false);
        assert!(counts(&mut s).0 == 3, "the picked line was not reflected: {:?}", counts(&mut s));
        let (min, max) = box_of(&mut s);
        assert!((min[0] + 30.0).abs() < 1e-6 && (max[0] - 30.0).abs() < 1e-6, "the reflection does not stand opposite the original: the drawing runs from {} to {} across", min[0], max[0]);
    }
}

probe! {
    /// DELETE: what is picked goes, and the rest stays.
    fn delete_takes_away_what_is_picked() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (20.0, 0.0));
        line(&mut s, (0.0, 10.0), (20.0, 10.0));
        pick(&mut s, 10.0, 10.0, false);
        take(&mut s, "tb-delete-hint");
        assert!(counts(&mut s).0 == 1, "the picked line was not taken away: {:?}", counts(&mut s));
        assert!(box_is(&mut s, [0.0, 0.0], [20.0, 0.0]), "the line that was not picked went with it: the drawing runs {:?}", box_of(&mut s));
    }
}

probe! {
    /// MOVE: what is picked goes from the base point to the target, and nothing is added.
    fn move_carries_what_is_picked_from_one_point_to_another() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (20.0, 0.0));
        pick(&mut s, 10.0, 0.0, false);
        take(&mut s, "tb-move-hint");
        s.click_on_sketch(0.0, 0.0);
        s.click_on_sketch(10.0, 10.0);
        assert!(counts(&mut s).0 == 1, "moving added geometry: {:?}", counts(&mut s));
        assert!(box_is(&mut s, [10.0, 10.0], [30.0, 10.0]), "the line did not move by (10, 10): it runs {:?}", box_of(&mut s));
    }
}

probe! {
    /// COPY: the original stays where it was and a copy stands at the target.
    fn copy_leaves_the_original_and_puts_a_copy_at_the_target() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (20.0, 0.0));
        pick(&mut s, 10.0, 0.0, false);
        take(&mut s, "tb-copy-hint");
        s.click_on_sketch(0.0, 0.0);
        s.click_on_sketch(10.0, 10.0);
        assert!(counts(&mut s).0 == 2, "there is no copy: {:?}", counts(&mut s));
        assert!(box_is(&mut s, [0.0, 0.0], [30.0, 10.0]), "the original and its copy do not stand 10 by 10 apart: {:?}", box_of(&mut s));
    }
}

probe! {
    /// ROTATE: what is picked turns about the centre by the angle that was typed.
    fn rotate_turns_what_is_picked_by_the_angle_typed() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (20.0, 0.0));
        pick(&mut s, 10.0, 0.0, false);
        take(&mut s, "tb-rotate-hint");
        s.click_on_sketch(0.0, 0.0);
        let angle = s.word("sk-angle-placeholder");
        s.fill_hinted(&angle, "90").key(Key::Enter);
        assert!(counts(&mut s).0 == 1, "turning the line added geometry: {:?}", counts(&mut s));
        assert!(box_is(&mut s, [0.0, 0.0], [0.0, 20.0]), "the line did not turn a quarter about the origin: it runs {:?}", box_of(&mut s));
    }
}

probe! {
    /// A LINEAR PATTERN: the picked line is laid out three times along X, ten apart, and the pattern can be opened
    /// again by a double click on one of the copies.
    fn a_linear_pattern_lays_out_copies_along_a_direction() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (5.0, 0.0));
        pick(&mut s, 2.5, 0.0, false);
        take(&mut s, "tb-lin-array-hint");
        let count = s.word("opt-count");
        s.fill(&count, "3");
        let fields = bar_fields(&mut s);
        fill_widget(&mut s, &fields[1], "10");
        fill_widget(&mut s, &fields[2], "0");
        s.key(Key::Enter).key(Key::Enter); // the first Enter only leaves the field - see the check below
        assert!(counts(&mut s).0 == 3, "three copies were asked for: {:?}", counts(&mut s));
        assert!(box_is(&mut s, [0.0, 0.0], [25.0, 0.0]), "the copies do not stand 10 apart along X: {:?}", box_of(&mut s));
        // opened again by a double click on a copy: the bar comes back with the count that was set
        let at = s.on_sketch(22.5, 0.0);
        s.double_click(at);
        let again = bar_fields(&mut s);
        assert!(again.first().map(|w| w.value.clone()).unwrap_or_default() == "3", "the pattern did not open again with its own count: the bar shows {:?}", again.iter().map(|w| w.value.clone()).collect::<Vec<_>>());
    }
}

probe! {
    /// A CIRCULAR PATTERN: the picked line is laid out four times about the origin, a quarter turn apart.
    fn a_circular_pattern_lays_out_copies_about_a_centre() {
        let mut s = empty_sketch();
        line(&mut s, (10.0, 0.0), (20.0, 0.0));
        pick(&mut s, 15.0, 0.0, false);
        take(&mut s, "tb-circ-array-hint");
        let count = s.word("opt-count");
        s.fill(&count, "4");
        let fields = bar_fields(&mut s);
        fill_widget(&mut s, &fields[1], "360");
        s.click_on_sketch(0.0, 0.0); // the centre to turn about
        s.key(Key::Enter).key(Key::Enter);
        assert!(counts(&mut s).0 == 4, "four copies were asked for: {:?}", counts(&mut s));
        assert!(box_is(&mut s, [-20.0, -20.0], [20.0, 20.0]), "the copies do not stand a quarter turn apart about the origin: {:?}", box_of(&mut s));
    }
}

probe! {
    /// THE ENTER THE BAR ASKS FOR APPLIES AT ONCE: "set the step and count above, press Enter" - one press, not two.
    fn the_enter_the_bar_asks_for_applies_at_once() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (5.0, 0.0));
        pick(&mut s, 2.5, 0.0, false);
        take(&mut s, "tb-lin-array-hint");
        let count = s.word("opt-count");
        s.fill(&count, "3");
        let fields = bar_fields(&mut s);
        fill_widget(&mut s, &fields[1], "10");
        s.key(Key::Enter);
        assert!(counts(&mut s).0 == 3, "one Enter after the numbers did not make the pattern: {:?} instead of 3 lines; the status line says {:?}", counts(&mut s), s.status());
    }
}

probe! {
    /// A TRIM AFTER A NEW PROJECT IN THE SAME WINDOW takes the piece clicked, as it does in a window just opened: a
    /// rectangle drawn, File -> New project, then a circle of 10 with a level line through it, and the lower half of
    /// the circle trimmed - one line and one arc left.
    fn a_trim_after_a_new_project_takes_the_piece_clicked() {
        let mut s = qymcad::Session::start();
        qymcad_acceptance::build::into_the_first_part(&mut s);
        let xy = s.word("plane-xy-table");
        s.press_word(&xy);
        draw(&mut s, "tb-rect-hint", &[(0.0, 0.0), (40.0, 30.0)]);
        let finish = s.word("wb-finish");
        s.press_word(&finish);
        let (file, new) = (s.word("menu-file"), s.word("file-new"));
        s.menu(&[&file, &new]);
        let dont = s.word("nav-dont-save");
        if let Some(b) = s.find(&dont, qymcad::pos2(0.0, 0.0)) {
            s.click(b.center());
        }
        qymcad_acceptance::build::into_the_first_part(&mut s);
        let xy = s.word("plane-xy-table");
        s.press_word(&xy);
        qymcad_acceptance::build::circle(&mut s, (0.0, 0.0), (10.0, 0.0));
        line(&mut s, (-20.0, 0.0), (20.0, 0.0));
        take(&mut s, "tb-trim-hint");
        s.click_on_sketch(0.0, -10.0);
        let sk = s.document().sketches[0].clone();
        assert!((sk.lines, sk.arcs, sk.circles) == (1, 1, 0), "the trim left {} lines, {} arcs and {} circles, not one line and one arc; the status line says {:?}", sk.lines, sk.arcs, sk.circles, s.status());
    }
}

probe! {
    /// SHIFT JOINS THE SET: three lines of a triangle named with Shift are three corners, cut by one answer.
    /// Reported: a shape whose corners were all wanted had to be rounded one corner at a time, each one its own value
    /// typed into a field that opened over a different corner every time.
    fn three_lines_named_with_shift_are_three_corners_cut_together() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (60.0, 0.0));
        line(&mut s, (60.0, 0.0), (30.0, 50.0));
        line(&mut s, (30.0, 50.0), (0.0, 0.0)); // a closed contour of three lines
        assert_eq!(counts(&mut s).0, 3, "setup: the triangle is not three lines: {:?}", counts(&mut s));
        take(&mut s, "tb-chamfer-sketch-hint");
        s.click_on_sketch(30.0, 0.0); // the middle of the first line
        let corner = s.on_sketch(45.0, 25.0); // the middle of the second: the field of their corner opens
        s.click(corner);
        let field = field_near(&mut s, corner);
        pick_shift(&mut s, 15.0, 25.0); // and with Shift the middle of the third, which closes the contour
        assert!(s.status().contains('3'), "the set was said to hold {:?} corners, not the three the contour makes", s.status());
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).0 == 6, "the three corners of the triangle were not cut by one answer: {:?}", counts(&mut s));
    }
}

probe! {
    /// FOUR LINES ON ONE POINT ARE TAKEN TWO AT A TIME AS THEY WERE NAMED: the third joins a corner two lines have
    /// already made and makes none of its own, and the fourth makes the second corner - with the third, not with the
    /// first. Reported: every pair of the four was a corner there, so naming a shape round a cross cut places nobody
    /// had asked for.
    fn the_third_line_of_a_cross_adds_no_corner_and_the_fourth_makes_the_second() {
        let mut s = empty_sketch();
        let middle = cross_of_four_lines(&mut s); // four spokes on the point (0, 0)
        assert_eq!(counts(&mut s).0, 4, "setup: the four spokes are not four lines: {:?}", counts(&mut s));
        take(&mut s, "tb-chamfer-sketch-hint");
        s.click_on_sketch(middle[0].0, middle[0].1); // the first spoke: it is half of a corner and waits
        let corner = s.on_sketch(middle[1].0, middle[1].1); // and the second: their corner at (0, 0) opens its field
        s.click(corner);
        let field = field_near(&mut s, corner);
        pick_shift(&mut s, middle[2].0, middle[2].1); // a third spoke on that point
        assert!(s.status().contains('1'), "a third line made a corner of its own, beside the one the first two made: {:?}", s.status());
        pick_shift(&mut s, middle[3].0, middle[3].1); // and the fourth, which makes the second corner with the third
        assert!(s.status().contains('2'), "the fourth line did not make the second corner: {:?}", s.status());
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).0 == 6, "four lines with two corners cut should be six, and the point was cut more than twice: {:?}", counts(&mut s));
    }
}

probe! {
    /// A LINE NAMED WITH SHIFT THAT MEETS NOTHING OF THE SET STANDS THERE AND WAITS: it is half of the next corner,
    /// and the corner named before it does not go off the sheet. Reported: the preview of the first corner was taken
    /// down by a line a person had only meant to put in the set.
    fn a_line_named_with_shift_that_meets_nothing_waits_and_keeps_the_corner_standing() {
        let mut s = empty_sketch();
        line(&mut s, (60.0, 0.0), (0.0, 0.0));
        line(&mut s, (0.0, 0.0), (0.0, 60.0));
        line(&mut s, (140.0, 140.0), (120.0, 140.0)); // an L far away
        line(&mut s, (120.0, 140.0), (120.0, 160.0));
        take(&mut s, "tb-chamfer-sketch-hint");
        s.click_on_sketch(30.0, 0.0); // the middle of the first line
        let corner = s.on_sketch(0.0, 30.0); // and of the second: the field of their corner opens
        s.click(corner);
        let field = field_near(&mut s, corner);
        assert_eq!(field.kind, Kind::TextField, "the corner of the first two lines did not open its field");
        pick_shift(&mut s, 130.0, 140.0); // with Shift the middle of a line that meets neither of them
        let still = s.widgets().into_iter().filter(|w| w.kind == Kind::TextField && w.rect.top() > 120.0).collect::<Vec<_>>();
        assert_eq!(still.len(), 1, "the field of the corner went off the sheet when a line was put in the set beside it: {still:?}");
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).0 == 5, "the one corner that was named was not cut, or a corner was cut where no lines met: {:?}", counts(&mut s));
    }
}

probe! {
    /// THE POINT OF A CORNER SWITCHES THAT CORNER OFF, AND THE SAME POINT BRINGS IT BACK. The corner was said by
    /// its lines, and a point naming it again would be a second reading of the same place - so the click does not
    /// read it, it puts it away: the lines keep their ends, and a click on the same point brings that very corner
    /// back rather than asking for a new one.
    fn a_point_wearing_a_named_corner_switches_it_off_and_back_with_shift() {
        let mut s = empty_sketch();
        cross_of_four_lines(&mut s);
        take(&mut s, "tb-chamfer-sketch-hint");
        s.click_on_sketch(1500.0, 0.0); // the first spoke
        let corner = s.on_sketch(0.0, 1500.0); // and the second: their corner at (0, 0) opens its field
        s.click(corner);
        assert_eq!(field_near(&mut s, corner).kind, Kind::TextField, "the two spokes did not name their corner");
        pick_shift(&mut s, 0.0, 0.0); // with Shift the point where those two lines already made their corner
        assert!(s.status().contains("out of the set"), "the point of a corner already named by its lines did not put that corner away: {:?}", s.status());
        assert!(counts(&mut s).0 == 4, "the four lines were changed by putting a corner away: {:?}", counts(&mut s));
        pick_shift(&mut s, 0.0, 0.0); // and the same point again brings that very corner back
        assert!(s.status().contains('1'), "the corner that was put away did not come back on the same click: {:?}", s.status());
        // THE BOX WENT DOWN WITH THE CORNER - there was nothing left to type for - so it is opened again on the
        // two lines, and one answer cuts the corner that came back and no other
        s.click_on_sketch(1500.0, 0.0);
        let corner = s.on_sketch(0.0, 1500.0);
        s.click(corner);
        let field = field_near(&mut s, corner);
        fill_widget(&mut s, &field, "5");
        s.key(Key::Enter);
        assert!(counts(&mut s).0 == 5, "four lines with the one corner of them cut should be five, and the point was cut twice over: {:?}", counts(&mut s));
    }
}

probe! {
    /// THE LINES CHOSEN BEFORE THE TOOL ARE THE SET THEY WOULD HAVE BEEN WITH SHIFT, AND THE FIELD OPENS ON THE
    /// FIRST OF THEIR CORNERS. Reported: taking the chamfer with a contour selected dropped the selection, and the
    /// corners of that contour had to be named one by one again. So the choice is the set from the moment the tool
    /// is taken - and the tool cannot wait to be told which corner to type for, because a click without Shift is a
    /// single selection and forgets the choice: one answer cuts the whole contour.
    fn the_lines_chosen_before_the_tool_are_cut_as_the_set() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (60.0, 0.0));
        line(&mut s, (60.0, 0.0), (30.0, 50.0));
        line(&mut s, (30.0, 50.0), (0.0, 0.0)); // a closed contour of three lines
        pick(&mut s, 30.0, 0.0, false);
        pick(&mut s, 45.0, 25.0, true);
        pick(&mut s, 15.0, 25.0, true); // all three lines, chosen before the tool
        take(&mut s, "tb-chamfer-sketch-hint");
        let boxes: Vec<Widget> = s.widgets().into_iter().filter(|w| w.kind == Kind::TextField && w.rect.top() > 120.0).collect();
        assert_eq!(boxes.len(), 1, "the tool was taken with a contour chosen and opened no field to type the value in: {boxes:?}");
        fill_widget(&mut s, &boxes[0], "5");
        s.key(Key::Enter);
        assert_eq!(counts(&mut s).0, 6, "the three lines chosen before the tool made one corner instead of three: {:?}", counts(&mut s));
    }
}

probe! {
    /// A POINT THAT NAMED THE CORNER IN THE FIELD IS TAKEN BACK WITH IT, SHIFT IN HAND. Reported: naming a point
    /// again could only add a second reading of the same place, so the corner could be named but never un-named
    /// once the field was open.
    fn a_point_that_named_the_corner_is_taken_back_by_shift() {
        let mut s = empty_sketch();
        line(&mut s, (60.0, 0.0), (0.0, 0.0));
        line(&mut s, (0.0, 0.0), (0.0, 60.0));
        take(&mut s, "tb-chamfer-sketch-hint");
        let corner = s.on_sketch(0.0, 0.0);
        s.click(corner); // the point at the corner: it names the corner by itself and its field opens
        assert_eq!(field_near(&mut s, corner).kind, Kind::TextField, "the point did not name its corner");
        pick_shift(&mut s, 0.0, 0.0); // and the very same point with Shift: the corner and the choice go together
        let left = s.widgets().into_iter().filter(|w| w.kind == Kind::TextField && w.rect.top() > 120.0).collect::<Vec<_>>();
        assert!(left.is_empty(), "the corner stood on with its field after the point that named it was named again: {left:?}");
        assert!(counts(&mut s).0 == 2, "something was cut although no corner was left in hand: {:?}", counts(&mut s));
    }
}

probe! {
    /// TWO POINTS NAMED ALONG A CONTOUR, AND THEN THE LINES. Reported, with Shift held throughout: the first point
    /// named a corner, the second point round the contour named another, naming the line that touches the second
    /// point made no corner at all, and naming the line that touches the first line named made one - but not the
    /// corner between those two lines, and a fourth corner was cut that nobody had asked for. A point had let all
    /// of its lines into the pairing, so the line that came afterwards had a neighbour among lines already used.
    /// A point names the corner it was read at; a line named afterwards joins the line already standing with it.
    fn two_points_of_a_square_and_then_its_lines_are_three_corners() {
        let mut s = empty_sketch();
        let a = ((0.0, 0.0), (2000.0, 0.0));
        let b = ((2000.0, 0.0), (2000.0, 2000.0));
        let c = ((2000.0, 2000.0), (0.0, 2000.0));
        let d = ((0.0, 2000.0), (0.0, 0.0)); // a square, drawn a long way out so that no pick lands on the field
        line(&mut s, a.0, a.1);
        line(&mut s, b.0, b.1);
        line(&mut s, c.0, c.1);
        line(&mut s, d.0, d.1);
        assert_eq!(counts(&mut s).0, 4, "setup: the square is not four lines: {:?}", counts(&mut s));
        take(&mut s, "tb-chamfer-sketch-hint");
        let first = s.on_sketch(0.0, 0.0);
        s.click(first); // the first corner of the square: its field opens
        let field = field_near(&mut s, first);
        pick_shift(&mut s, 2000.0, 0.0); // and with Shift the second corner, round the contour
        assert!(s.status().contains('2'), "two points of a square named two corners: {:?}", s.status());
        pick_shift(&mut s, 2000.0, 1000.0); // a line that touches the second point: it is one of the lines there already
        assert!(s.status().contains('2'), "naming a line of a corner already in the set made a corner of its own: {:?}", s.status());
        pick_shift(&mut s, 1000.0, 2000.0); // and the line that touches the first line named: the corner between them
        assert!(s.status().contains('3'), "the two lines named by the hand did not make their corner, or a fourth corner was made: {:?}", s.status());
        fill_widget(&mut s, &field, "200");
        s.key(Key::Enter);
        assert_eq!(counts(&mut s).0, 7, "four lines with three corners cut should be seven: {:?}", counts(&mut s));
    }
}

probe! {
    /// A CORNER A POINT NAMED, AND THE LINES NAMED AFTERWARDS. Reported: a chamfer named by a point where three or
    /// more lines meet stayed on the pair the cursor had read, and when lines were named later and ran through that
    /// point, the chamfer was not built on them. The lines a person chose have the first word at the point where a
    /// corner was only read - which is said of the corner (`CornerMade`) and not by the colour drawn over it.
    fn the_lines_named_after_a_point_are_the_corner_where_they_run_through_it() {
        let mut s = empty_sketch();
        let middle = cross_of_four_lines(&mut s); // four spokes on the point (0, 0)
        take(&mut s, "tb-chamfer-sketch-hint");
        let origin = s.on_sketch(0.0, 0.0);
        pick_shift(&mut s, 0.0, 0.0); // the point four lines meet at: one corner of them is named and its field opens
        let field = field_near(&mut s, origin);
        assert_eq!(field.kind, Kind::TextField, "the point did not name a corner of the four lines standing at it");
        pick_shift(&mut s, middle[2].0, middle[2].1); // and then two of those lines, named on the side away from the field
        pick_shift(&mut s, middle[3].0, middle[3].1);
        assert!(s.status().contains('2'), "the two lines named did not make their corner at the point they run through, or the corner the point named was counted twice: {:?}", s.status());
        fill_widget(&mut s, &field, "300");
        s.key(Key::Enter);
        assert_eq!(counts(&mut s).0, 6, "four lines with two corners cut should be six: {:?}", counts(&mut s));
    }
}

probe! {
    /// A CLICK WITHOUT SHIFT IS A SINGLE SELECTION, AND A MULTI-SELECTION IS MADE WITH SHIFT. Reported: after two
    /// corners had been named with Shift, a click without Shift on a corner of its own cut the whole set with it,
    /// and there was no way to take one corner out of a set. So the plain click is the single one: it says which
    /// corner the value is for, and what was named with Shift is forgotten - the set is begun and left with Shift.
    fn a_click_without_shift_cuts_the_one_corner_and_forgets_the_set() {
        let mut s = empty_sketch();
        line(&mut s, (0.0, 0.0), (2000.0, 0.0));
        line(&mut s, (2000.0, 0.0), (2000.0, 2000.0));
        line(&mut s, (2000.0, 2000.0), (0.0, 2000.0));
        line(&mut s, (0.0, 2000.0), (0.0, 0.0)); // a square, drawn a long way out so that no pick lands on the field
        line(&mut s, (1400.0, 1400.0), (1800.0, 1400.0)); // and inside it an L of its own, whose corner is named
        line(&mut s, (1400.0, 1400.0), (1400.0, 1800.0)); // by a plain click at the end
        take(&mut s, "tb-chamfer-sketch-hint");
        pick_shift(&mut s, 0.0, 0.0); // the first point of the square: its corner and its field
        pick_shift(&mut s, 2000.0, 0.0); // and with Shift the second, round the contour
        assert!(s.status().contains('2'), "two points of a square named two corners: {:?}", s.status());
        let alone = s.on_sketch(1400.0, 1400.0);
        s.click(alone); // A CLICK WITHOUT SHIFT: one corner, and the set is forgotten
        let field = field_near(&mut s, alone);
        fill_widget(&mut s, &field, "200");
        s.key(Key::Enter);
        // six lines with ONE corner cut: the two points of the square named with Shift were not cut with it
        assert_eq!(counts(&mut s).0, 7, "a click without Shift cut the corners named with Shift along with its own: {:?}", counts(&mut s));
    }
}
