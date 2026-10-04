//! WHAT A SET OF PICKS MAKES OF CORNERS, AND WHAT MAY BE NAMED BESIDE IT.
//!
//! A pick is a line or a point, and a corner was named by one of the three windows of two picks that the tool
//! already reads. Shift makes a pick mean something else: it does not take the corner away and make a new one, it
//! joins the one in hand. So the set has to answer two questions before Enter: what corners do these picks make
//! together, and may this next pick be named with them at all.
//!
//! **A point is a corner before anything else is** - naming one lets in every line standing at it, so that a shape
//! can be named by clicking its vertices rather than hunting for its lines.
//!
//! **A point that already wears a corner of named lines may not be named again.** That corner was said by the
//! lines, and where four lines stand on one point the lines say WHICH of the corners at it is meant while the point
//! says only where. Naming it twice would be two readings of one place, and the second would quietly win.
use qymcad_core::feature::Purpose;
use qymcad_core::model::Project;

/// THE PAIRS ALONE, where what a corner is made of does not matter to the question.
fn pairs(corners: &[qymcad_ui_state::SetCorner]) -> Vec<(u64, u64)> {
    corners.iter().map(|c| c.pair).collect()
}

/// THE SAME PAIRS IN ONE ORDER: which of two lines came first is said by the order they were named, and a corner
/// is the same corner whichever way round it is written down.
fn same_corners(got: &[qymcad_ui_state::SetCorner], want: &[(u64, u64)]) -> bool {
    let mut g: Vec<(u64, u64)> = pairs(got).into_iter().map(|(a, b)| (a.min(b), a.max(b))).collect();
    g.sort_unstable();
    let mut w: Vec<(u64, u64)> = want.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
    w.sort_unstable();
    g == w
}

/// A closed square of four lines, and the four points of its corners. Returns the lines in the order of the
/// contour, so that `lines[i]` runs from `points[i]` to `points[i + 1]`.
fn square() -> (Project, usize, [u64; 4], [u64; 4]) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let at = [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)];
    let mut lines = [0u64; 4];
    for i in 0..4 {
        let (a, b) = (at[i], at[(i + 1) % 4]);
        lines[i] = p.add_line_entity(si, a.0, a.1, b.0, b.1, Purpose::Real);
    }
    p.regen_sketch(si);
    let mut points = [0u64; 4];
    for (i, (x, y)) in at.into_iter().enumerate() {
        points[i] = p.sketches[si].points.iter().find(|q| (q.x - x).abs() < 1e-6 && (q.y - y).abs() < 1e-6).expect("the square has that corner").id;
    }
    (p, si, lines, points)
}

/// Two squares sharing the point (20, 20), with the four lines that stand on it.
fn cross_at_one_point() -> (Project, usize, Vec<u64>, u64) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    for (a, b) in [
        ((0.0, 0.0), (20.0, 0.0)),
        ((20.0, 0.0), (20.0, 20.0)),
        ((20.0, 20.0), (0.0, 20.0)),
        ((0.0, 20.0), (0.0, 0.0)),
        ((20.0, 20.0), (40.0, 20.0)),
        ((40.0, 20.0), (40.0, 40.0)),
        ((40.0, 40.0), (20.0, 40.0)),
        ((20.0, 40.0), (20.0, 20.0)),
    ] {
        p.add_line_entity(si, a.0, a.1, b.0, b.1, Purpose::Real);
    }
    p.regen_sketch(si);
    let shared = p.sketches[si].points.iter().find(|q| (q.x - 20.0).abs() < 1e-6 && (q.y - 20.0).abs() < 1e-6).expect("the point the two squares share").id;
    let edges = p.vertex_edges(si, shared);
    assert_eq!(edges.len(), 4, "setup: four lines do not stand on the point");
    (p, si, edges, shared)
}

/// A triangle of three lines.
fn triangle() -> (Project, usize, Vec<u64>) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let ids: Vec<u64> =
        [((0.0, 0.0), (40.0, 0.0)), ((40.0, 0.0), (20.0, 30.0)), ((20.0, 30.0), (0.0, 0.0))].iter().map(|&((ax, ay), (bx, by))| p.add_line_entity(si, ax, ay, bx, by, Purpose::Real)).collect();
    p.regen_sketch(si);
    (p, si, ids)
}

#[test]
fn two_lines_named_with_shift_make_their_corner() {
    let (p, si, l) = triangle();
    let picks = vec![(1, l[0]), (1, l[1])];
    let made = qymcad_ui_state::corners_of_picks(&p, si, &picks);
    assert_eq!(made.len(), 1, "two lines of the contour make one corner and wait for the rest");
    assert!(p.corner_of_pair(si, made[0].pair.0, made[0].pair.1).is_some(), "the corner they name is a corner of the drawing");
}

#[test]
fn the_third_line_of_a_triangle_completes_it() {
    let (p, si, l) = triangle();
    let picks = vec![(1, l[0]), (1, l[1]), (1, l[2])];
    assert_eq!(qymcad_ui_state::corners_of_picks(&p, si, &picks).len(), 3, "a closed contour of three lines is three corners, cut by one answer");
}

#[test]
fn a_point_named_on_its_own_lets_in_the_lines_standing_at_it() {
    let (p, si, e, shared) = cross_at_one_point();
    // the point alone brings in all four lines, and four lines on one point are two corners, not six
    let made = qymcad_ui_state::corners_of_picks(&p, si, &[(0, shared)]);
    assert_eq!(made.len(), 2, "four lines on one point make two corners taken two at a time");
    assert!(pairs(&made).contains(&(e[0], e[1])) || pairs(&made).contains(&(e[1], e[0])), "the first two lines make the first corner");
}

#[test]
fn a_line_and_a_point_named_together_name_the_line_s_corner() {
    let (p, si, l) = triangle();
    let point = p.corner_of_pair(si, l[0], l[1]).expect("the two lines meet at a point");
    // the point named first, then the line: the lines of the point are let in, and the line joins them
    let made = qymcad_ui_state::corners_of_picks(&p, si, &[(0, point), (1, l[2])]);
    assert_eq!(made.len(), 3, "the whole contour is named by its point and one line");
}

#[test]
fn a_point_wearing_a_corner_of_named_lines_may_not_be_named_again() {
    let (p, si, e, shared) = cross_at_one_point();
    let named = vec![(1, e[0]), (1, e[1])];
    assert_eq!(qymcad_ui_state::corners_of_picks(&p, si, &named).len(), 1, "setup: the two lines made their corner");
    assert!(!qymcad_ui_state::pick_may_join(&p, si, &named, (0, shared)), "the point of a corner already named by its lines was let in as a second reading of the same place");
    assert!(qymcad_ui_state::pick_may_join(&p, si, &named, (1, e[2])), "a line may always be named beside the ones in hand");
}

#[test]
fn a_point_with_no_corner_of_named_lines_at_it_may_be_named() {
    let (p, si, e, shared) = cross_at_one_point();
    // nothing named yet: the point is a corner before anything else is, and it may be named
    assert!(qymcad_ui_state::pick_may_join(&p, si, &[], (0, shared)), "a point that was nobody's corner was refused: a shape cannot be named by its vertices");
    let elsewhere = p.sketches[si].points.iter().find(|q| q.id != shared).expect("the squares have other points").id;
    assert!(qymcad_ui_state::pick_may_join(&p, si, &[(1, e[0]), (1, e[1])], (0, elsewhere)), "a point with no corner of the named lines at it was refused for no reason");
}

#[test]
fn the_value_of_a_set_is_held_by_its_tightest_corner() {
    let (p, si, l) = triangle();
    // a long thin triangle: the sharp corner takes far less than the blunt ones
    let picks = vec![(1, l[0]), (1, l[1]), (1, l[2])];
    let made = qymcad_ui_state::corners_of_picks(&p, si, &picks);
    let limit = qymcad_ui_state::corners_limit(&p, si, &made, false).expect("corners of a triangle hold a value");
    for c in &made {
        let one = p.corner_limit_of_pair(si, p.corner_of_pair(si, c.pair.0, c.pair.1).unwrap(), c.pair, false).expect("a corner holds a value");
        assert!(limit <= one + 1e-9, "one value cuts all of them together, so the set takes no more than the tightest of them");
    }
    assert!(limit > 0.0, "the corners of a triangle hold a positive value");
}

#[test]
fn a_set_of_no_corners_holds_no_value() {
    let (p, si, e, _) = cross_at_one_point();
    assert_eq!(qymcad_ui_state::corners_limit(&p, si, &[], false), None, "nothing to cut holds no value");
    let one = [qymcad_ui_state::SetCorner { pair: (e[0], e[1]), made: qymcad_ui_state::CornerMade::Lines }];
    assert_eq!(qymcad_ui_state::corners_limit(&p, si, &one, true), p.corner_limit_of_pair(si, p.corner_of_pair(si, e[0], e[1]).unwrap(), (e[0], e[1]), true), "one corner of a set is held by itself");
}

/// A FIXED CORNER IS NOT AN APPLIED ONE: what is remembered is the pair of lines, and it is that pair which the
/// next pick cannot move.
#[test]
fn a_fixed_corner_keeps_the_two_lines_it_was_named_by() {
    let (p, si, e, _) = cross_at_one_point();
    let picks = vec![(1, e[0]), (1, e[1]), (1, e[2]), (1, e[3])];
    let free = qymcad_ui_state::corners_of_the_set(&p, si, &picks, &[], &[], None);
    assert_eq!(pairs(&free), vec![(e[0], e[1]), (e[2], e[3])], "four lines on one point are two corners taken two at a time");
    // the FIRST corner is fixed, and the second is still a corner: fixing one pair does not cost the drawing the
    // other pair at the same point
    let held = vec![(e[2], e[3])];
    let mixed = qymcad_ui_state::corners_of_the_set(&p, si, &picks, &held, &[], None);
    assert_eq!(pairs(&mixed), vec![(e[2], e[3]), (e[0], e[1])], "a fixed corner keeps its place and the rest of the set is untouched");
    // a fixed corner whose point is no longer a corner is not offered: the drawing does not hold what was remembered
    assert_eq!(qymcad_ui_state::corners_of_the_set(&p, si, &picks, &[(e[0], e[2])], &[], None).len(), 3, "a pair that is not a corner of the drawing was remembered as one");
}

/// THE LINES THAT CARRY A FIXED CORNER ARE THE WAY BACK TO IT.
#[test]
fn a_line_that_carries_a_fixed_corner_is_the_way_back_to_it() {
    let (p, si, l) = triangle();
    let fixed = vec![(l[0], l[1])];
    assert!(qymcad_ui_state::carries_a_fixed_corner(&fixed, (1, l[0])), "the first line of a fixed corner did not carry it");
    assert!(qymcad_ui_state::carries_a_fixed_corner(&fixed, (1, l[1])), "the second line of a fixed corner did not carry it");
    assert!(!qymcad_ui_state::carries_a_fixed_corner(&fixed, (1, l[2])), "a line that takes no part in the fixed corner carried it");
    assert!(!qymcad_ui_state::carries_a_fixed_corner(&fixed, (0, p.corner_of_pair(si, l[0], l[1]).unwrap())), "a point carries a corner of lines only by its lines");
    // nothing fixed, nothing carried
    assert!(!qymcad_ui_state::carries_a_fixed_corner(&[], (1, l[0])), "every line carries a fixed corner when none is fixed");
}

/// A POINT NAMED WITH SHIFT IS READ IN THE DIRECTION OF THE CURSOR, the way the first corner of the tool is: where
/// four lines stand on it, the point says where and the side the pointer is on says which of the corners there.
#[test]
fn a_point_named_with_shift_is_read_in_the_direction_of_the_cursor() {
    let (p, si, e, shared) = cross_at_one_point();
    let (cx, cy) = p.sketches[si].points.iter().find(|q| q.id == shared).map(|q| (q.x, q.y)).expect("the shared point stands on the sheet");
    let (west, east) = ((cx - 10.0, cy), (cx + 10.0, cy));
    let aimed = [(shared, e[0])];
    let left = qymcad_ui_state::corners_of_picks_aimed(&p, si, &[(0, shared)], &aimed, Some(west));
    let right = qymcad_ui_state::corners_of_picks_aimed(&p, si, &[(0, shared)], &aimed, Some(east));
    assert_eq!(left.len(), 1, "the corner of the point read through a line is one corner, not every pair of them");
    assert_ne!(pairs(&left), pairs(&right), "the point alone read the same corner on both sides of it: the cursor has no word in it");
    // A CORNER READ FROM A POINT IS A CORNER OF A POINT, and that is said of it: no line was named here, and the
    // way it was named is a property of the corner rather than a colour drawn over it.
    assert!(left.iter().all(|c| c.made == qymcad_ui_state::CornerMade::Point), "a corner nobody picked a line of was made of a point");
    // and with no line named the point brings the lines in and pairs them as they come - which is what a point
    // among two lines means
    assert_eq!(qymcad_ui_state::corners_of_picks_aimed(&p, si, &[(0, shared)], &[], None).len(), 2, "four lines on a point are two corners");
}

/// THE LINES OF A POINT STAND AT THAT POINT, and a line named after them joins the one already there.
///
/// Reported on a square, with Shift held throughout: the first point was named and a corner was drawn there, the
/// second point round the contour was named and a corner was drawn there too, then the line touching the second
/// point was named and NO corner was made - and naming the line that touches the first one named made a corner
/// where none was asked for. A point had let all of its lines into the pairing, so the line that came afterwards
/// had a neighbour to pair with among lines already used, and the corners of the set were not the corners of the
/// drawing a person was naming.
#[test]
fn a_line_named_at_a_point_of_the_set_joins_the_line_already_standing_there() {
    let (p, si, l, points) = square();
    let aimed = qymcad_ui_state::line_through_point(&p, si, points[0]).expect("a corner has a line through it");
    let reads = [(points[0], aimed)];
    // THE TWO POINTS NAMED ALONG THE CONTOUR: each names the corner of its own point and nothing else.
    let two = qymcad_ui_state::corners_of_picks_aimed(&p, si, &[(0, points[0]), (0, points[1])], &reads, None);
    assert!(same_corners(&two, &[(l[3], l[0]), (l[0], l[1])]), "two points of a square name the two corners they stand on - and no others: {:?}", pairs(&two));
    // THE LINE THAT TOUCHES THE SECOND POINT is a line of the corner already named there: naming it adds nothing.
    let with_it = qymcad_ui_state::corners_of_picks_aimed(&p, si, &[(0, points[0]), (0, points[1]), (1, l[1])], &reads, None);
    assert!(same_corners(&with_it, &pairs(&two)), "a line of a corner already in the set was named again and made a corner of its own: {:?}", pairs(&with_it));
    // THE LINE THAT TOUCHES THE FIRST LINE NAMED makes the corner between them, and there is no fourth corner:
    // the line that meets it at the far corner of the square is not one anybody named.
    let all = qymcad_ui_state::corners_of_picks_aimed(&p, si, &[(0, points[0]), (0, points[1]), (1, l[1]), (1, l[2])], &reads, None);
    assert_eq!(all.len(), 3, "three corners were named and a fourth was made of lines nobody chose");
    assert!(all.iter().any(|c| c.pair == (l[1], l[2])), "the corner between the two lines named by the hand is a corner of the set");
}

/// WHERE A LINE NAMED BY THE HAND STANDS AT A POINT A CORNER WAS READ AT, THE HAND WINS.
#[test]
fn a_line_named_afterwards_takes_the_place_of_the_corner_a_point_named() {
    let (p, si, e, shared) = cross_at_one_point();
    let (cx, cy) = p.sketches[si].points.iter().find(|q| q.id == shared).map(|q| (q.x, q.y)).expect("the shared point stands on the sheet");
    let reads = [(shared, e[0])];
    let west = (cx - 10.0, cy);
    let alone = qymcad_ui_state::corners_of_picks_aimed(&p, si, &[(0, shared)], &reads, Some(west));
    assert_eq!(alone.len(), 1, "the point alone names one corner of the four lines standing at it");
    // THE LINES NAMED AFTERWARDS, where they run through that point: the corner is built on them.
    let picks = [(0, shared), (1, e[2]), (1, e[3])];
    let named = qymcad_ui_state::corners_of_picks_aimed(&p, si, &picks, &reads, Some(west));
    assert_eq!(named.len(), 2, "the corner the point named and the corner of the named lines are two, not one taken twice");
    let of_lines = named.iter().find(|c| c.pair == (e[2], e[3])).expect("the lines named by the hand did not make their corner at the point they run through");
    assert_eq!(of_lines.made, qymcad_ui_state::CornerMade::Lines, "the corner of the lines a hand named is a corner of lines");
    // A LINE NAMED THAT DOES NOT STAND AT THAT POINT leaves the corner the cursor read alone, and it is still a
    // corner of a point - the property follows the corner and is not decided by a colour drawn over it.
    let far = p.sketches[si].entities.iter().map(|x| x.id).find(|id| !e.contains(id)).expect("the squares have lines that do not stand on the shared point");
    let beside = qymcad_ui_state::corners_of_picks_aimed(&p, si, &[(0, shared), (1, far)], &reads, Some(west));
    let read = beside.iter().find(|c| p.corner_of_pair(si, c.pair.0, c.pair.1) == Some(shared)).expect("the corner of the point was moved off its point by a line named elsewhere");
    assert_eq!(read.made, qymcad_ui_state::CornerMade::Point, "a corner read off a point is one of a point until a line is named at that point");
}
