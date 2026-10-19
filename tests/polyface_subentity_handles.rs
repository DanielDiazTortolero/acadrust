//! A polyface mesh read from a DXF keeps its vertex and face records' handles and layers and its SEQEND's handle.
//! The DXF reader built every polyface vertex and face record with an empty common part and dropped the SEQEND's
//! handle, so a DXF->DWG conversion gave them new handles and moved them to layer 0 (MILI's CB-0058: the write gate's
//! second readers saw the records re-handled; DWG TrueView drew the written mesh the same as AutoCAD's own reading of
//! the DXF, its layer then having the same colour as layer 0).
use std::io::Cursor;

use acadrust::entities::{EntityType, PolyfaceFace, PolyfaceMesh, PolyfaceVertex};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter, Handle, Layer, Vector3};

struct Ids {
    mesh: Handle,
    vertices: Vec<Handle>,
    faces: Vec<Handle>,
    seqend: Handle,
}

/// A 2x1 quad grid (6 vertices, 2 faces) on layer MESH, every record with its own handle.
fn drawing() -> (CadDocument, Ids) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.layers.add(Layer::new("MESH")).unwrap();
    let mut mesh = PolyfaceMesh::new();
    mesh.common.layer = "MESH".into();
    let mut vertices = Vec::new();
    for (x, y) in [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (0.0, 10.0), (10.0, 10.0), (20.0, 10.0)] {
        let mut v = PolyfaceVertex::new(Vector3::new(x, y, 0.0));
        v.common.handle = doc.allocate_handle();
        v.common.layer = "MESH".into();
        vertices.push(v.common.handle);
        mesh.add_vertex(v);
    }
    let mut faces = Vec::new();
    for quad in [PolyfaceFace::quad(1, 2, 5, 4), PolyfaceFace::quad(2, 3, 6, 5)] {
        let mut f = quad;
        f.common.handle = doc.allocate_handle();
        f.common.layer = "MESH".into();
        faces.push(f.common.handle);
        mesh.add_face(f);
    }
    let seqend = doc.allocate_handle();
    mesh.seqend_handle = Some(seqend);
    let handle = doc.add_entity(EntityType::PolyfaceMesh(mesh)).unwrap();
    (doc, Ids { mesh: handle, vertices, faces, seqend })
}

fn mesh(doc: &CadDocument, h: Handle) -> &PolyfaceMesh {
    match doc.get_entity(h).expect("the mesh") {
        EntityType::PolyfaceMesh(m) => m,
        other => panic!("not a polyface mesh: {other:?}"),
    }
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc).write_to_vec().unwrap();
    DxfReader::from_reader(Cursor::new(bytes)).unwrap().read().unwrap()
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(doc).unwrap())).read().unwrap()
}

fn assert_kept(doc: &CadDocument, ids: &Ids, how: &str) {
    let m = mesh(doc, ids.mesh);
    let vh: Vec<Handle> = m.vertices.iter().map(|v| v.common.handle).collect();
    let fh: Vec<Handle> = m.faces.iter().map(|f| f.common.handle).collect();
    assert_eq!(vh, ids.vertices, "{how}: vertex handles");
    assert_eq!(fh, ids.faces, "{how}: face record handles");
    assert!(m.vertices.iter().all(|v| v.common.layer == "MESH"), "{how}: a vertex left layer MESH");
    assert!(m.faces.iter().all(|f| f.common.layer == "MESH"), "{how}: a face record left layer MESH");
    assert_eq!(m.seqend_handle, Some(ids.seqend), "{how}: the SEQEND's handle");
}

#[test]
fn a_dxf_read_keeps_the_records_handles_and_layers() {
    let (doc, ids) = drawing();
    assert_kept(&through_dxf(&doc), &ids, "DXF read");
}

#[test]
fn a_dxf_to_dwg_conversion_keeps_them() {
    let (doc, ids) = drawing();
    assert_kept(&through_dwg(&through_dxf(&doc)), &ids, "DXF -> DWG");
}
