//! WHAT THE VIEW TURNS ABOUT, by the setting: the middle of the view, or the point under the pointer where the turn
//! begins (`OrbitAbout`). The point is taken once, when the turn begins, and held for as long as it lasts - a point
//! taken anew every frame would be a point under a pointer that moves, and the view would swim.
use egui::{Rect, Response};

/// Where the turn of this frame is about: `None` for the middle of the view.
pub(crate) fn pivot(pn: &qymcad_ui_state::Painting, rect: Rect, ctx: &egui::Context, resp: &Response) -> Option<[f64; 3]> {
    let id = egui::Id::new("orbit-about-pivot");
    let turning = pn.set.mouse_nav.rotates().iter().chain(pn.set.mouse_nav.tilts().iter()).any(|g| g.active(ctx, resp));
    if pn.set.orbit_about != qymcad_ui_state::OrbitAbout::Pointer || !turning {
        ctx.data_mut(|d| d.remove::<[f64; 3]>(id));
        return None;
    }
    if let Some(p) = ctx.data(|d| d.get_temp::<[f64; 3]>(id)) {
        return Some(p);
    }
    // where the turn began: the press of a drag, or the pointer where a turn without a button starts moving
    let at = ctx.input(|i| i.pointer.press_origin()).or_else(|| resp.hover_pos())?;
    let p = super::look_at_point::point_under(pn, rect, at);
    ctx.data_mut(|d| d.insert_temp(id, p));
    Some(p)
}
