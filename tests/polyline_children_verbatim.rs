//! An untouched heavy polyline (2D, 3D, polyface or polygon mesh) is copied verbatim with its VERTEX records and
//! SEQEND, like an untouched INSERT with its ATTRIBs (insert_children_verbatim.rs). The writer re-encoded every one
//! from the model, where a 2D polyline's vertices and every polyline's SEQEND have no handle, so each save gave them
//! new handles (MILI's CB-0069) and any imprecision in the reading reached the copy. The records still carry the bytes
//! they were read from: a polyline the edit didn't touch keeps them; a touched one is encoded as before.
use std::io::Cursor;

use acadrust::entities::{EntityType, PolyfaceFace, PolyfaceMesh, PolyfaceVertex, Polyline2D, Vertex2D};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfVersion, Handle, Vector3};

fn reread(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(doc).unwrap())).read().unwrap()
}

fn polyline_drawing() -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut pl = Polyline2D::new();
    for (x, y) in [(0.0, 0.0), (10.0, 0.0), (10.0, 5.0)] {
        pl.add_vertex(Vertex2D::new(Vector3::new(x, y, 0.0)));
    }
    let h = doc.add_entity(EntityType::Polyline2D(pl)).unwrap();
    (doc, h)
}

fn mesh_drawing() -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut mesh = PolyfaceMesh::new();
    for (x, y) in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)] {
        mesh.add_vertex(PolyfaceVertex::new(Vector3::new(x, y, 0.0)));
    }
    mesh.add_face(PolyfaceFace::quad(1, 2, 3, 4));
    let h = doc.add_entity(EntityType::PolyfaceMesh(mesh)).unwrap();
    (doc, h)
}

fn first_x(doc: &CadDocument, h: Handle) -> f64 {
    match doc.get_entity(h).unwrap() {
        EntityType::Polyline2D(p) => p.vertices[0].location.x,
        EntityType::PolyfaceMesh(m) => m.vertices[0].location.x,
        other => panic!("unexpected {other:?}"),
    }
}

/// The read drawing with its polyline's first vertex moved in the model while the polyline keeps the bytes it was
/// read with: what an untouched polyline looks like to the writer when its model and its bytes could disagree.
fn model_moved_bytes_kept(mut doc: CadDocument, h: Handle) -> CadDocument {
    let raw = doc.get_entity(h).unwrap().common().raw_record.clone();
    assert!(raw.is_some(), "the polyline keeps its bytes");
    match doc.get_entity_mut(h).unwrap() {
        EntityType::Polyline2D(p) => {
            p.vertices[0].location.x = 99.0;
            p.common.raw_record = raw;
        }
        EntityType::PolyfaceMesh(m) => {
            m.vertices[0].location.x = 99.0;
            m.common.raw_record = raw;
        }
        other => panic!("unexpected {other:?}"),
    }
    doc
}

#[test]
fn an_untouched_2d_polyline_is_copied_not_re_encoded() {
    let (doc, h) = polyline_drawing();
    let out = reread(&model_moved_bytes_kept(reread(&doc), h));
    assert_eq!(first_x(&out, h), 0.0, "the polyline was re-encoded from the model, not copied");
}

#[test]
fn an_untouched_polyface_mesh_is_copied_not_re_encoded() {
    let (doc, h) = mesh_drawing();
    let out = reread(&model_moved_bytes_kept(reread(&doc), h));
    assert_eq!(first_x(&out, h), 0.0, "the mesh was re-encoded from the model, not copied");
}

#[test]
fn a_polyline_copied_twice_keeps_every_byte() {
    let (doc, h) = polyline_drawing();
    let once = reread(&doc);
    let twice = reread(&once);
    assert_eq!(
        once.get_entity(h).unwrap().common().raw_record,
        twice.get_entity(h).unwrap().common().raw_record,
        "a second save changed the polyline's record (its vertex or SEQEND handles)"
    );
}

#[test]
fn a_touched_polyline_is_encoded_from_the_model() {
    let (doc, h) = polyline_drawing();
    let mut read = reread(&doc);
    if let Some(EntityType::Polyline2D(p)) = read.get_entity_mut(h) {
        p.vertices[0].location.x = 99.0;
    }
    assert_eq!(first_x(&reread(&read), h), 99.0);
}
