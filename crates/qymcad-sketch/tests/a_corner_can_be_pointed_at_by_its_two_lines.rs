//! A CORNER IS POINTED AT BY ITS TWO LINES, NOT ONLY BY ITS POINT.
//!
//! The fillet and the chamfer took a corner only when the cursor found the vertex itself, so the corner of a long
//! edge had to be hit within a few pixels of the point. Two lines that meet at a corner ARE that corner: naming
//! them is naming it. And a pair that shares no corner is not a corner at all - the selection is dropped rather
//! than kept waiting for a radius that could never be taken off anything.
use qymcad_core::feature::Purpose;
use qymcad_core::model::Project;
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

/// The selection as it stands: two lines, one line, three lines, or a point where a line should be.
fn sel(items: &[(u8, u64)]) -> SketchSelection {
    SketchSelection { items: items.to_vec(), ..Default::default() }
}

#[test]
fn two_lines_that_share_a_corner_offer_it() {
    let (p, si, (a, b, _)) = an_angle_with_a_stranger();
    let mut s = sel(&[(1, a), (1, b)]);
    let pid = qymcad_ui_state::corner_of_selection(&p, si, &mut s);
    assert_eq!(pid, p.shared_vertex(si, a, b), "the corner the two lines share was not the one offered: {pid:?}");
    assert!(pid.is_some(), "two lines meeting at a corner are that corner, and it was not offered");
    assert_eq!(s.items.len(), 2, "the pair was dropped: it must stay lit while the value is decided, for a person to see WHICH corner is being rounded");
}

#[test]
fn two_lines_that_share_no_point_lose_the_selection() {
    let (p, si, (a, _b, c)) = an_angle_with_a_stranger();
    let mut s = sel(&[(1, a), (1, c)]);
    assert!(p.shared_vertex(si, a, c).is_none(), "setup: these two lines meet nowhere");
    assert_eq!(qymcad_ui_state::corner_of_selection(&p, si, &mut s), None, "a pair with no corner in common offered one");
    assert!(s.items.is_empty(), "the pair with no corner stayed lit: the tool would wait on something that cannot come");
}

#[test]
fn more_than_two_lines_lose_the_selection() {
    let (p, si, (a, b, c)) = an_angle_with_a_stranger();
    assert!(p.shared_vertex(si, a, b).is_some(), "setup: the first two lines do meet");
    let mut s = sel(&[(1, a), (1, b), (1, c)]);
    assert_eq!(qymcad_ui_state::corner_of_selection(&p, si, &mut s), None, "three lines offered a corner");
    assert!(s.items.is_empty(), "three lines stayed lit: a corner is two lines, and the third says nothing about which");
}

#[test]
fn a_point_where_a_line_belongs_loses_the_selection() {
    let (p, si, (a, _b, _c)) = an_angle_with_a_stranger();
    let corner = p.sketches[si].points.first().map(|q| q.id).expect("the corner point");
    let mut s = sel(&[(1, a), (0, corner)]);
    assert_eq!(qymcad_ui_state::corner_of_selection(&p, si, &mut s), None, "a line and a point offered a corner");
    assert!(s.items.is_empty(), "a line and a point stayed lit: nothing can be rounded off one line and one point");
}

#[test]
fn one_line_is_the_first_half_of_a_corner_and_stays_chosen() {
    let (p, si, (a, _b, _c)) = an_angle_with_a_stranger();
    let mut s = sel(&[(1, a)]);
    assert_eq!(qymcad_ui_state::corner_of_selection(&p, si, &mut s), None, "one line alone offered a corner");
    assert_eq!(s.items, vec![(1, a)], "the first half of a corner was dropped: it is what the next click answers");
}

#[test]
fn the_second_line_offers_the_corner_between_them() {
    let (p, si, (a, b, _c)) = an_angle_with_a_stranger();
    let mut s = SketchSelection::default();
    assert_eq!(qymcad_ui_state::corner_line_clicked(&p, si, a, &mut s), None, "the first line offered a corner by itself");
    assert_eq!(s.items, vec![(1, a)], "the first line was not taken as the first half of a corner");
    let pid = qymcad_ui_state::corner_line_clicked(&p, si, b, &mut s);
    assert_eq!(pid, p.shared_vertex(si, a, b), "the second line did not answer with the corner they share: {pid:?}");
    assert!(pid.is_some(), "two lines clicked in turn did not offer their corner");
}

#[test]
fn a_second_line_without_a_corner_becomes_the_first() {
    let (mut p, si, (a, _b, c)) = an_angle_with_a_stranger();
    let mut s = SketchSelection::default();
    qymcad_ui_state::corner_line_clicked(&p, si, a, &mut s);
    assert_eq!(qymcad_ui_state::corner_line_clicked(&p, si, c, &mut s), None, "two lines with nothing in common offered a corner");
    assert_eq!(s.items, vec![(1, c)], "the line clicked did not become the first of the next pair: the search would have stopped there");
    // and the search goes on: a neighbour of the new first line still finds its corner
    let d = p.add_line_entity(si, 100.0, 80.0, 100.0, 100.0, Purpose::Real);
    p.regen_sketch(si);
    assert_eq!(qymcad_ui_state::corner_line_clicked(&p, si, d, &mut s), p.shared_vertex(si, c, d), "the search did not carry on from the line that was clicked last");
}

#[test]
fn the_same_line_clicked_twice_lets_itself_go() {
    let (p, si, (a, _b, _c)) = an_angle_with_a_stranger();
    let mut s = SketchSelection::default();
    qymcad_ui_state::corner_line_clicked(&p, si, a, &mut s);
    assert_eq!(qymcad_ui_state::corner_line_clicked(&p, si, a, &mut s), None, "a line offered a corner with itself");
    assert!(s.items.is_empty(), "the line stayed lit: the second click on it was meant to take the choice back");
}
