//! THE CORNER IS NAMED BY THE LAST TWO PICKS, AND BY WHAT THEY ARE.
//!
//! The corner of two lines is a corner anybody can see, and the fillet has always been able to take it off. What it
//! could not do was go round a drawing picking corner after corner without stopping at each one to start over, and at
//! a point where four lines meet it could not tell which of the four corners a person meant.
//!
//! So a pick is a line or a point, and the window is the LAST TWO OF THEM: every pick pushes the older one out, so
//! the second of a pair is the first of the next one and three lines in a row are two corners in a row. What the window
//! names is what is offered, and there are three answers - two edges name their own corner; an edge and a point name
//! the corners that edge takes part in; a point alone names every corner at it. A window that names nothing is the
//! newest pick alone, so the search goes on where the hand went on.
use qymcad_core::feature::Purpose;
use qymcad_core::model::{ChamferLegs, Project};
use qymcad_ui_state::SketchSelection;

/// An L of two lines meeting at the origin, and a third line far away with a corner of its own.
fn an_angle_with_a_stranger() -> (Project, usize, (u64, u64, u64)) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let a = p.add_line_entity(si, 0.0, 0.0, 40.0, 0.0, Purpose::Real);
    let b = p.add_line_entity(si, 0.0, 0.0, 0.0, 30.0, Purpose::Real);
    let c = p.add_line_entity(si, 80.0, 80.0, 100.0, 80.0, Purpose::Real);
    p.regen_sketch(si);
    (p, si, (a, b, c))
}

/// TWO SQUARES SHARING ONE POINT AT (20, 20): four lines meet there, and four corners are in it.
///
/// The four lines by the direction they run from the point - `west` to the left, `right` down, `east` to the right,
/// `north` up - and the squares between them: the near one between `west` and `right`, the far one between `east`
/// and `north`. Those two pairs are the corners a pointer inside the near square and a pointer inside the far one
/// name.
fn two_squares_at_one_point() -> (Project, usize, u64, [u64; 4]) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let line = |p: &mut Project, a: (i32, i32), b: (i32, i32)| p.add_line_entity(si, a.0 as f64, a.1 as f64, b.0 as f64, b.1 as f64, Purpose::Real);
    let west = line(&mut p, (20, 20), (0, 20));
    let right = line(&mut p, (20, 0), (20, 20));
    line(&mut p, (0, 20), (0, 0));
    line(&mut p, (0, 0), (20, 0));
    let east = line(&mut p, (20, 20), (40, 20));
    let north = line(&mut p, (20, 40), (20, 20));
    line(&mut p, (40, 20), (40, 40));
    line(&mut p, (40, 40), (20, 40));
    p.regen_sketch(si);
    let shared = p.sketches[si].points.iter().find(|q| (q.x - 20.0).abs() < 1e-6 && (q.y - 20.0).abs() < 1e-6).map(|q| q.id).expect("the shared point");
    assert_eq!(p.vertex_edges(si, shared).len(), 4, "setup: four lines do not meet at the point");
    (p, si, shared, [west, right, east, north])
}

/// The point of a sketch at `(x, y)`.
fn point_at(p: &Project, si: usize, x: f64, y: f64) -> u64 {
    p.sketches[si].points.iter().find(|q| (q.x - x).abs() < 1e-6 && (q.y - y).abs() < 1e-6).map(|q| q.id).unwrap_or_else(|| panic!("there is no point at ({x}, {y})"))
}

/// Whether a pair is the line and the other, in either order.
fn pair_has(pair: (u64, u64), line: u64) -> bool {
    pair.0 == line || pair.1 == line
}

/// Whether two pairs are the same two lines, in either order.
fn same_pair(a: (u64, u64), b: (u64, u64)) -> bool {
    (a.0 == b.0 && a.1 == b.1) || (a.0 == b.1 && a.1 == b.0)
}

/// TWO PICKS IN TURN OFFER THE CORNER THEY SHARE, and both stand lit while the value is decided.
#[test]
fn two_lines_that_share_a_corner_offer_it() {
    let (p, si, (a, b, _c)) = an_angle_with_a_stranger();
    let mut s = SketchSelection::default();
    assert_eq!(qymcad_ui_state::corner_picked(&p, si, (1, a), &mut s), None, "the first line offered a corner by itself");
    assert_eq!(s.items, vec![(1, a)], "the first line was not taken as the first half of a corner: it is what the next pick answers");
    let choice = qymcad_ui_state::corner_picked(&p, si, (1, b), &mut s).expect("two lines picked in turn did not offer their corner");
    assert_eq!(choice.pid, point_at(&p, si, 0.0, 0.0), "the corner offered is not the one the two lines share");
    assert_eq!(choice.edges, Some((a, b)), "the pair was not named, so the cursor would be asked which corner of the point is meant: {choice:?}");
    assert_eq!(choice.through, None, "two lines name the corner between them and nothing else: {choice:?}");
    assert_eq!(s.items, vec![(1, a), (1, b)], "both sides of the corner must stand lit while the value is decided: only {:?} is lit", s.items);
}

/// THREE LINES IN A ROW ARE TWO CORNERS IN A ROW: the second of a pair stays lit after the corner is taken off, so the
/// next pick pairs with it instead of starting over.
#[test]
fn the_second_of_a_pair_is_the_first_of_the_next() {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let a = p.add_line_entity(si, 0.0, 0.0, 20.0, 0.0, Purpose::Real);
    let b = p.add_line_entity(si, 20.0, 0.0, 20.0, 20.0, Purpose::Real);
    let c = p.add_line_entity(si, 20.0, 20.0, 40.0, 20.0, Purpose::Real);
    p.regen_sketch(si);
    let mut s = SketchSelection::default();
    assert!(qymcad_ui_state::corner_picked(&p, si, (1, a), &mut s).is_none(), "setup: one line offers no corner");
    let first = qymcad_ui_state::corner_picked(&p, si, (1, b), &mut s).expect("the first two lines offer their corner");
    let first_pair = first.edges.expect("two lines name the pair");
    assert!(same_pair(first_pair, (a, b)), "the first corner is not the one between the first two lines: {first_pair:?}");
    // THE CORNER IS TAKEN OFF, as Enter does it, and what stands lit is the second of the pair
    assert!(p.chamfer_lines_of_pair(si, first_pair, ChamferLegs::equal(4.0), None), "the first corner did not apply");
    let second = qymcad_ui_state::corner_picked(&p, si, (1, c), &mut s).expect("the third line did not pair with the second one");
    let second_pair = second.edges.expect("two lines name the pair");
    assert!(pair_has(second_pair, b) && pair_has(second_pair, c), "the third line did not pair with the line that was left lit: {second_pair:?}");
}

/// A WINDOW THAT NAMES NOTHING IS THE NEWEST PICK ALONE: two lines with nothing in common are no corner, and the one
/// picked last stands as the first of the next pair, so the search goes on where the hand went on.
#[test]
fn a_window_that_names_nothing_is_the_newest_pick_alone() {
    let (mut p, si, (a, _b, c)) = an_angle_with_a_stranger();
    let mut s = SketchSelection::default();
    qymcad_ui_state::corner_picked(&p, si, (1, a), &mut s);
    assert_eq!(qymcad_ui_state::corner_picked(&p, si, (1, c), &mut s), None, "two lines with nothing in common offered a corner");
    assert_eq!(s.items, vec![(1, c)], "the line picked last did not become the first of the next pair: only {:?} is lit", s.items);
    // and the search goes on: a neighbour of the new first line still finds its corner
    let d = p.add_line_entity(si, 100.0, 80.0, 100.0, 100.0, Purpose::Real);
    p.regen_sketch(si);
    assert_eq!(qymcad_ui_state::corner_picked(&p, si, (1, d), &mut s).map(|c| c.pid), p.shared_vertex(si, c, d), "the search did not carry on from the line that was picked last");
}

/// THE SAME PICK TWICE TAKES THE CHOICE BACK: a line with itself is no corner of anything, and a person who picks the
/// same line again means to have nothing chosen rather than to have chosen it.
#[test]
fn the_same_pick_twice_takes_the_choice_back() {
    let (p, si, (a, _b, _c)) = an_angle_with_a_stranger();
    let pid = point_at(&p, si, 0.0, 0.0);
    let mut s = SketchSelection::default();
    qymcad_ui_state::corner_picked(&p, si, (1, a), &mut s);
    assert_eq!(qymcad_ui_state::corner_picked(&p, si, (1, a), &mut s), None, "a line offered a corner with itself");
    assert!(s.items.is_empty(), "the line stayed lit: the second pick on it was meant to take the choice back");
    // a point goes the same way
    assert!(qymcad_ui_state::corner_picked(&p, si, (0, pid), &mut s).is_some(), "setup: a point alone names the corner at it");
    assert_eq!(qymcad_ui_state::corner_picked(&p, si, (0, pid), &mut s), None, "a point offered a corner with itself");
    assert!(s.items.is_empty(), "the point stayed lit: the second pick on it was meant to take the choice back");
}

/// TWO HALVES OF ONE STRAIGHT LINE: they share a point and make an angle of 180 degrees, which is not a corner.
/// Reported: a line drawn in two pieces could not be taken apart at the joint, and neither could the search go on.
#[test]
fn a_straight_joint_of_two_lines_is_not_a_corner() {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let half1 = p.add_line_entity(si, 0.0, 0.0, 20.0, 0.0, Purpose::Real);
    let half2 = p.add_line_entity(si, 20.0, 0.0, 40.0, 0.0, Purpose::Real);
    p.regen_sketch(si);
    assert_eq!(p.corner_of_pair(si, half1, half2), None, "setup: two halves of one line are taken for a corner");
    let mut s = SketchSelection::default();
    qymcad_ui_state::corner_picked(&p, si, (1, half1), &mut s);
    assert_eq!(qymcad_ui_state::corner_picked(&p, si, (1, half2), &mut s), None, "a straight angle of 180 degrees offered a corner to round");
    assert_eq!(s.items, vec![(1, half2)], "the second half did not become the first of the next pair: the search stopped on a joint that has no angle in it");
}

/// A POINT ALONE NAMES EVERY CORNER AT IT: the pair is left to the cursor and no line narrows the choice. Both squares
/// are among the answers, which is what "every corner" has to mean at a point of four lines.
#[test]
fn a_point_alone_offers_every_corner_at_it() {
    let (p, si, shared, [west, right, east, north]) = two_squares_at_one_point();
    let mut s = SketchSelection::default();
    let choice = qymcad_ui_state::corner_picked(&p, si, (0, shared), &mut s).expect("a point alone named no corner: it is what a point is for");
    assert_eq!(choice.pid, shared, "the corner is not at the point that was picked");
    assert_eq!(choice.edges, None, "a point alone names the point, not a pair of lines: {choice:?}");
    assert_eq!(choice.through, None, "a point alone narrows nothing: {choice:?}");
    assert_eq!(s.items, vec![(0, shared)], "the point picked does not stand lit: {:?} is lit", s.items);
    let near = qymcad_ui_state::corner_pair_now(&p, si, shared, None, None, false, Some((19.0, 19.0))).expect("a pointer in the near square named no corner");
    let far = qymcad_ui_state::corner_pair_now(&p, si, shared, None, None, false, Some((21.0, 21.0))).expect("a pointer in the far square named no corner");
    assert!(same_pair(near, (west, right)), "the pointer inside the near square did not name the corner it stands in: {near:?}");
    assert!(same_pair(far, (east, north)), "the pointer inside the far square did not name the corner it stands in: {far:?}");
}

/// A LINE AND A POINT NAME THE CORNERS OF THAT LINE: the point says where the corner is, the line says which of the
/// corners there is meant, so only the ones that line takes part in are among the answers - and the pointer anywhere
/// round the point still gets one of them.
#[test]
fn a_line_and_a_point_name_the_corners_of_that_line() {
    let (p, si, shared, edges) = two_squares_at_one_point();
    for line in edges {
        let mut s = SketchSelection::default();
        qymcad_ui_state::corner_picked(&p, si, (1, line), &mut s);
        let choice = qymcad_ui_state::corner_picked(&p, si, (0, shared), &mut s).unwrap_or_else(|| panic!("a line and the point it meets at offered no corner"));
        assert_eq!(choice.pid, shared, "the corner is not at the point that was picked");
        assert_eq!(choice.edges, None, "a line and a point name no pair: the pointer says which corner of the point: {choice:?}");
        assert_eq!(choice.through, Some(line), "the corner was not narrowed to the line that was picked first: {choice:?}");
        assert_eq!(s.items, vec![(1, line), (0, shared)], "both picks must stand lit while the value is decided: {:?} is lit", s.items);
        for at in [(19.0, 19.0), (21.0, 21.0), (10.0, 21.0), (21.0, 10.0), (19.0, 10.0)] {
            let pair = qymcad_ui_state::corner_pair_now(&p, si, shared, None, Some(line), false, Some(at)).unwrap_or_else(|| panic!("the pointer at {at:?} named no corner of the line"));
            assert!(pair_has(pair, line), "the corner offered does not take part in the line that was named: {pair:?} against the line {line}");
        }
    }
}

/// THE CORNER THE POINTER NAMED LAST STANDS WHILE IT IS AWAY. Reported: a line and then the point at its end were
/// picked, the pointer chose which way the corner goes, and taking the pointer away to type the radius put the
/// preview back on the other side of the line.
#[test]
fn the_corner_the_pointer_named_last_stands_while_it_is_away() {
    let (p, si, shared, [west, right, _east, _north]) = two_squares_at_one_point();
    // A LINE AND ITS POINT: the pointer inside the near square names the corner on that side of the line
    let named = qymcad_ui_state::corner_pair_now(&p, si, shared, None, Some(right), false, Some((19.0, 19.0))).expect("the pointer named no corner of the line");
    assert!(same_pair(named, (west, right)), "the pointer inside the near square did not name the corner on that side of the line: {named:?}");
    // and with the pointer away - in the field, typing the radius - that corner is still the one in force
    let away = qymcad_ui_state::corner_pair_now(&p, si, shared, Some(named), Some(right), false, None).expect("no corner stands while the pointer is away");
    assert!(same_pair(away, named), "the preview went back to another corner while the pointer was away: {away:?} against the one that was named {named:?}");
    // A POINT ALONE keeps its corner the same way
    let alone = qymcad_ui_state::corner_pair_now(&p, si, shared, None, None, false, Some((21.0, 21.0))).expect("the pointer named no corner of the point");
    let alone_away = qymcad_ui_state::corner_pair_now(&p, si, shared, Some(alone), None, false, None).expect("no corner stands while the pointer is away");
    assert!(same_pair(alone_away, alone), "the corner of the point changed while the pointer was away: {alone_away:?} against {alone:?}");
    // and a pair that was never a corner at the point is not taken for the one standing
    let at_the_point = p.vertex_edges(si, shared);
    let stranger = p.sketches[si]
        .entities
        .iter()
        .find(|e| !at_the_point.contains(&e.id) && matches!(e.kind, qymcad_core::model::EntityKind::Line { .. }))
        .map(|e| e.id)
        .expect("a line that does not touch the point");
    let in_force = qymcad_ui_state::corner_pair_now(&p, si, shared, Some((stranger, right)), Some(right), false, None).expect("no corner stands while the pointer is away");
    assert!(!pair_has(in_force, stranger), "a pair that is no corner at the point was taken for the corner in force: {in_force:?}");
}

/// AND THE POINTER SAYS WHICH SIDE OF THAT LINE: the corners of a line at a point of four are the two sectors either
/// side of it, and a pointer in one of them is in that corner.
#[test]
fn the_corner_of_a_line_follows_the_pointer_to_the_side_of_it() {
    let (p, si, shared, [west, right, east, _north]) = two_squares_at_one_point();
    // THE NEAR SQUARE is the sector on one side of `right`, and the pointer in it names the corner of that side
    let near = qymcad_ui_state::corner_pair_now(&p, si, shared, None, Some(right), false, Some((19.0, 19.0))).expect("a pointer in the near square named no corner of the line");
    assert!(same_pair(near, (west, right)), "the pointer inside the near square did not name the corner on that side of the line: {near:?}");
    // THE FAR SQUARE lies in a sector that belongs to two other edges, so with this line named the corner offered is
    // the nearer of the line's two - and never one the line takes no part in
    let far = qymcad_ui_state::corner_pair_now(&p, si, shared, None, Some(right), false, Some((21.0, 21.0))).expect("a pointer in the far square named no corner of the line");
    assert!(same_pair(far, (right, east)), "the pointer in the far square is nearer the middle of the sector after the line, so that is the corner of the line it names: {far:?}");
    // without the line named, the very same pointer names the far square's own corner, which the line is not part of
    let unlined = qymcad_ui_state::corner_pair_now(&p, si, shared, None, None, false, Some((21.0, 21.0))).expect("a pointer in the far square named no corner");
    assert!(!pair_has(unlined, right), "a corner of the line was offered where none was asked for: {unlined:?}");
    assert!(!same_pair(unlined, far), "the line named first made no difference to the corner that was offered: {unlined:?}");
}

/// A LINE AND A POINT WITH NOTHING IN COMMON NAME NO CORNER, so the point picked last stands alone - and a point alone
/// names the corner at it, which is what the search goes on with.
#[test]
fn a_line_and_a_stranger_point_leave_the_point_alone() {
    let (p, si, (a, _b, _c)) = an_angle_with_a_stranger();
    let pid = point_at(&p, si, 80.0, 80.0);
    let mut s = SketchSelection::default();
    qymcad_ui_state::corner_picked(&p, si, (1, a), &mut s);
    let choice = qymcad_ui_state::corner_picked(&p, si, (0, pid), &mut s).expect("the point picked alone named no corner");
    assert_eq!(choice.pid, pid, "the corner offered is not at the point that was picked last");
    assert_eq!(choice.through, None, "the line that was picked first does not narrow a corner at a point it does not reach: {choice:?}");
    assert_eq!(s.items, vec![(0, pid)], "the point picked last does not stand alone: {:?} is lit", s.items);
}

/// TWO POINTS SHARE NO GEOMETRY TO ROUND: a line, a point and then another point leaves the point picked last alone,
/// and the corner at it is the one that is offered.
#[test]
fn two_points_leave_the_point_picked_last_alone() {
    let (p, si, (a, _b, _c)) = an_angle_with_a_stranger();
    let first = point_at(&p, si, 0.0, 0.0);
    let last = point_at(&p, si, 100.0, 80.0);
    let mut s = SketchSelection::default();
    qymcad_ui_state::corner_picked(&p, si, (1, a), &mut s);
    let choice = qymcad_ui_state::corner_picked(&p, si, (0, first), &mut s).expect("setup: a line and the point at its end name a corner");
    assert_eq!(choice.pid, first, "the corner offered is not at the point of the first pair");
    let choice = qymcad_ui_state::corner_picked(&p, si, (0, last), &mut s).expect("two points named no corner at all");
    assert_eq!(choice.pid, last, "the corner offered is not at the point picked last: {choice:?}");
    assert_eq!(s.items, vec![(0, last)], "two points share no geometry to round, so the point picked last stands alone: {:?} is lit", s.items);
}

/// FOUR LINES THROUGH ONE POINT: a pair of two edges named by the person IS the corner, and the pointer cannot move
/// it to another of the four — reported as the fillet jumping away from the point although only two lines had been
/// picked.
#[test]
fn a_named_pair_stands_against_the_pointer() {
    let (p, si, shared, [west, right, _east, _north]) = two_squares_at_one_point();
    for at in [(19.0, 19.0), (21.0, 21.0), (25.0, 20.0), (15.0, 20.0), (20.0, 25.0), (20.0, 15.0)] {
        let pair = qymcad_ui_state::corner_pair_now(&p, si, shared, Some((west, right)), None, true, Some(at)).expect("the named pair named no corner");
        assert!(same_pair(pair, (west, right)), "the pointer at {at:?} moved the corner off the two lines that were picked: {pair:?}");
    }
    // and with the pointer away from the sheet it stands too, so the corner does not move under the value being typed
    let away = qymcad_ui_state::corner_pair_now(&p, si, shared, Some((west, right)), None, true, None).expect("the named pair named no corner");
    assert!(same_pair(away, (west, right)), "the corner changed with no pointer over the sheet: {away:?}");
}

/// A CORNER ALREADY CHOSEN IS OFFERED THE MOMENT THE TOOL IS TAKEN: the lines were picked before the tool was, so
/// asking for the corner again would be asking what has already been said.
#[test]
fn two_chosen_lines_offer_their_corner_at_once() {
    let (p, si, (a, b, c)) = an_angle_with_a_stranger();
    let corner = point_at(&p, si, 0.0, 0.0);
    let mut s = SketchSelection { items: vec![(1, a), (1, b)], ..Default::default() };
    let choice = qymcad_ui_state::corner_of_selection(&p, si, &mut s).expect("two chosen lines sharing a corner offered none");
    assert_eq!((choice.pid, choice.edges), (corner, Some((a, b))), "the corner the two chosen lines make was not the one offered: {choice:?}");
    assert_eq!(s.items, vec![(1, a), (1, b)], "the pair was dropped: it must stay lit while the value is decided, for a person to see WHICH corner is being rounded");
    // and a choice that names no corner is not dropped: it is the SET the mode works on, and the lines in it that
    // do meet at a corner are cut as if they had been named with Shift while the tool was on
    let mut s = SketchSelection { items: vec![(1, a), (1, c)], ..Default::default() };
    assert_eq!(qymcad_ui_state::corner_of_selection(&p, si, &mut s), None, "a pair with no corner in common offered one");
    assert_eq!(s.items, vec![(1, a), (1, c)], "the pair with no corner in common was dropped: it is the set, and the tool asks for the corner rather than losing the choice");
    // a third thing beside two lines says nothing about WHICH corner is meant, but the three of them are still the set
    let mut s = SketchSelection { items: vec![(1, a), (1, b), (1, c)], ..Default::default() };
    assert_eq!(qymcad_ui_state::corner_of_selection(&p, si, &mut s), None, "three lines offered a corner");
    assert_eq!(s.items, vec![(1, a), (1, b), (1, c)], "three chosen lines were dropped: a contour chosen before the tool is the set the mode cuts");
    // one line alone is the first half of a corner, not a refusal
    let mut s = SketchSelection { items: vec![(1, a)], ..Default::default() };
    assert_eq!(qymcad_ui_state::corner_of_selection(&p, si, &mut s), None, "one line alone offered a corner");
    assert_eq!(s.items, vec![(1, a)], "the first half of a corner was dropped: it is what the next pick answers");
}

/// THE LINES CHOSEN BEFORE THE TOOL ARE THE SET: the ones among them that meet at a corner are corners of it, exactly
/// as if they had been named with Shift while the mode was on.
#[test]
fn the_lines_chosen_before_the_tool_are_a_set_of_corners() {
    let (p, si, (a, b, c)) = an_angle_with_a_stranger();
    let picks = vec![(1, a), (1, b), (1, c)];
    assert_eq!(qymcad_ui_state::corners_of_picks(&p, si, &picks).len(), 1, "of three chosen lines only the two that meet make a corner");
    assert_eq!(qymcad_ui_state::corners_of_picks(&p, si, &picks[..2]).len(), 1, "the corner of the first two is the one the field opens on");
}
