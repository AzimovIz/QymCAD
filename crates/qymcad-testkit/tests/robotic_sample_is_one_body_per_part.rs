//! THE ROBOT SAMPLE: EVERY PART ONE BODY. Four copies of part 5 held two bodies each - two chains of their own, an
//! extrusion cut by a body of revolution and an extrusion cut by a sketch - and the owner decided on 27.09 they are one
//! part: the two are united, as a person unites them, by a body boolean at the end of the part. The recipe is run by
//! hand and writes the sample; the check below it reads the sample as it stands.
use qymcad_core::model::{Id, Project};

const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/robotic_2.qcad");

/// The bodies of part `part` a later step does not use up.
fn live(p: &Project, part: Id) -> Vec<Id> {
    let consumed: std::collections::HashSet<Id> = p.timeline.iter().flat_map(|n| n.kind.consumed()).collect();
    p.component_bodies(part).into_iter().filter(|b| !consumed.contains(b) && p.mesh_index(*b).is_some()).collect()
}

/// The parts of the sample holding more than one body - before the recipe, the four copies of part 5.
fn parts_of_two_bodies(p: &Project) -> Vec<Id> {
    p.components.iter().filter(|c| c.kind == qymcad_core::feature::ComponentKind::Part && live(p, c.id).len() > 1).map(|c| c.id).collect()
}

#[test]
#[ignore = "writes examples/robotic_2.qcad"]
fn unite_the_two_bodies_of_part_5() {
    let mut p = qymcad_io::load_project(SAMPLE).expect("the sample opens");
    let parts = parts_of_two_bodies(&p);
    assert_eq!(parts.len(), 4, "the sample holds {} parts of two bodies, not the four copies of part 5", parts.len());
    for &part in &parts {
        let bodies = live(&p, part);
        assert_eq!(bodies.len(), 2, "{part}: {bodies:?}");
        // the chain cut by the body of revolution is the base, the one cut by a sketch is united into it
        let (a, b) = (bodies[1].max(bodies[0]), bodies[1].min(bodies[0]));
        p.set_active_component(Some(part));
        p.add_body_boolean(a, b, 1);
    }
    // only these four parts are built again: the rest of the sample keeps the geometry its owner saved
    for n in p.timeline.iter_mut() {
        n.dirty = n.parent.is_some_and(|c| parts.contains(&c));
    }
    let (report, _) = qymcad_testkit::regenerate_dirty_with_shapes(&mut p, Default::default());
    let own: Vec<_> = report.errors.iter().filter(|(id, _)| p.timeline.iter().any(|n| n.id == *id && n.parent.is_some_and(|c| parts.contains(&c)))).collect();
    assert!(own.is_empty(), "the parts of part 5 went red: {own:?}");
    for &part in &parts {
        assert_eq!(live(&p, part).len(), 1, "{part} still holds {:?}", live(&p, part));
    }
    qymcad_io::save_project(&p, SAMPLE).expect("the sample is written");
}

/// No part of the sample holds more than one body.
#[test]
fn every_part_of_the_robot_sample_is_one_body() {
    let p = qymcad_io::load_project(SAMPLE).expect("the sample opens");
    let many: Vec<(Id, Vec<Id>)> = parts_of_two_bodies(&p).into_iter().map(|c| (c, live(&p, c))).collect();
    assert!(many.is_empty(), "parts of the sample holding more than one body: {many:?}");
}
