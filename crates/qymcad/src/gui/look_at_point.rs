//! LOOKING AT A POINT: a short click of the middle button, under the layouts whose programs have it, brings the point
//! under the pointer to the middle of the view and turns the view about it from then on - the point of the model hit
//! there, or, over empty space, the point of the plane through the present centre square to the view.
use egui::{Rect, Response};
use qymcad_core::model::{Id, Project};

/// Where the view is to look, once a middle click asks for it under `nav`.
pub(crate) fn look_at(pn: &qymcad_ui_state::Painting, nav: qymcad_ui_state::MouseNav, resp: &Response, rect: Rect) -> Option<[f64; 3]> {
    if !nav.middle_click_looks() || !resp.middle_clicked() {
        return None;
    }
    Some(point_under(pn, rect, resp.interact_pointer_pos()?))
}

/// THE POINT UNDER SCREEN `at` that the view looks at or turns about: the point of the model hit there, or, over empty
/// space, the point of the plane through the present centre square to the view.
pub(crate) fn point_under(pn: &qymcad_ui_state::Painting, rect: Rect, at: egui::Pos2) -> [f64; 3] {
    if let Some((_, _, w)) = qymcad_pick::pick_face_ray(pn, rect, at) {
        return w;
    }
    let basis = pn.cam.basis();
    let centre = qymcad_ui_state::Screen { cam: &pn.cam, set: pn.set, rect, basis: &basis }.at(pn.cam.target).0;
    let (right, up, _) = basis;
    let (dx, dy, k) = ((at.x - centre.x) as f64, (at.y - centre.y) as f64, 1.0 / pn.cam.scale as f64);
    let t = pn.cam.target;
    [t[0] + (right[0] * dx - up[0] * dy) * k, t[1] + (right[1] * dx - up[1] * dy) * k, t[2] + (right[2] * dx - up[2] * dy) * k]
}

/// The status line of a context stepped into: its name, as the window names it.
pub(crate) fn context_is(project: &Project, active_path: &[Id]) -> String {
    let ctx = qymcad_ui_state::current_ctx_id(active_path, project);
    let name = project.components.iter().find(|c| c.id == ctx).map(|c| crate::i18n::name(&c.name)).unwrap_or_default();
    crate::i18n::tr1("g-context-is", "name", &name)
}
