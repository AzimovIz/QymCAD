//! THE CURSOR NAMES ONE OF THE CORNERS AT A POINT ONLY WHILE IT STANDS NEAR THAT POINT.
//!
//! Where four lines meet at one point there are four corners there, and the cursor says which one is meant by the
//! side of the point it stands on. That is a good answer close to the point and a meaningless one far from it: with
//! the cursor read wherever it happened to be, the corner changed as the pointer crossed the sheet, so the radius
//! being typed in the field would be cut at a corner nobody was looking at - and the sheet shows the corner it is
//! showing, so what Enter cut was not what was on screen.
//!
//! The reach is the aim the point itself is caught from, `Grab::Point`, a shade wider - the person who aimed there has
//! the corner open and must be able to move it - and no wider: what is wanted is a small circle round the point, not
//! the drawing.
use qymcad_core::feature::Purpose;
use qymcad_core::model::Project;
use qymcad_ui_state::grab::{grab, Grab};
use qymcad_ui_state::Settings;

/// A sketch with one line from the origin to (20, 0), and the point at its far end: the point the cursor stands near.
fn a_point_and_its_id() -> (Project, usize, u64) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    p.add_line_entity(si, 0.0, 0.0, 20.0, 0.0, Purpose::Real);
    p.regen_sketch(si);
    let pid = p.sketches[si].points.iter().find(|q| (q.x - 20.0).abs() < 1e-9).map(|q| q.id).expect("the far end of the line is a point");
    (p, si, pid)
}

/// STANDING BESIDE THE POINT, THE CURSOR SPEAKS; A POINTER ELSEWHERE ON THE SHEET SAYS NOTHING.
#[test]
fn a_cursor_far_from_the_point_does_not_name_a_corner() {
    let (p, si, pid) = a_point_and_its_id();
    let reach = qymcad_ui_state::corner_reach(&Settings::default());
    let beside = (20.0, reach as f64 / 2.0); // half the reach away: inside it
    assert_eq!(qymcad_ui_state::corner_cursor(&p, si, pid, Some(beside), reach, 1.0), Some(beside), "the cursor standing beside the point was not heard: it is the only thing that says which corner is meant");
    assert_eq!(qymcad_ui_state::corner_cursor(&p, si, pid, Some((20.0 + 40.0, 0.0)), reach, 1.0), None, "a cursor forty units away named a corner: the corner followed the pointer across the sheet");
    assert_eq!(qymcad_ui_state::corner_cursor(&p, si, pid, None, reach, 1.0), None, "no cursor at all named a corner");
}

/// THE REACH IS A SMALL CIRCLE ROUND THE POINT, AND A SHADE WIDER THAN THE POINT IS CAUGHT FROM.
#[test]
fn the_reach_is_narrow_and_follows_the_picking() {
    let mut set = Settings::default();
    let point = grab(&set, Grab::Point);
    let reach = qymcad_ui_state::corner_reach(&set);
    assert!(reach > point, "the reach of the cursor is not wider than the aim the point itself is caught from ({reach} against {point}): a person who aimed there has the corner open and cannot move it");
    assert!(reach < point * 1.5, "the reach of the cursor is not a small circle round the point any more ({reach} against {point}): the corner would follow the pointer across the drawing");
    // and it is the aim of the person, not a number in place: precise aiming narrows it, coarse widens it
    for level in [0u8, 1, 2] {
        set.pick_precision = level;
        assert_eq!(qymcad_ui_state::corner_reach(&set), grab(&set, Grab::Corner), "the reach of the cursor is not the aim of the person at pick precision {level}");
        assert!(qymcad_ui_state::corner_reach(&set) > grab(&set, Grab::Point), "at pick precision {level} the cursor reaches less far than the point is caught from");
    }
}

/// THE REACH IS A CIRCLE ON THE SHEET, SO IT IS THE SAME ONE AT EVERY ZOOM.
#[test]
fn the_reach_is_the_same_pixels_at_every_zoom() {
    let (p, si, pid) = a_point_and_its_id();
    let reach = qymcad_ui_state::corner_reach(&Settings::default());
    // SIXTY UNITS FROM THE POINT: out of reach on a sheet at a pixel to the unit, inside it on a sheet drawn at a
    // tenth of a pixel to the unit. The reach is a circle on the screen - the same one whatever the zoom.
    let away = (80.0, 0.0);
    assert_eq!(qymcad_ui_state::corner_cursor(&p, si, pid, Some(away), reach, 1.0), None, "a cursor sixty pixels away named a corner");
    assert_eq!(qymcad_ui_state::corner_cursor(&p, si, pid, Some(away), reach, 0.1), Some(away), "the same cursor six pixels away did not name a corner: the reach is a circle on the sheet, not a circle in the drawing");
}