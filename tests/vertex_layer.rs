//! A heavy polyline's vertices keep their layer through a DWG read. The DWG builder gave every 3D polyline vertex an
//! empty layer name and every polygon-mesh vertex a fresh entity on layer 0, so the DXF written from a DWG listed
//! VERTEX records on layer '' (no such layer: AutoCAD's own DXF of its sample drawing has all twelve on '0') or on
//! the wrong one (MILI's CB-0016). The builder now keeps each vertex's own layer, and the DXF writer gives a vertex
//! without one its polyline's layer.
use std::io::Cursor;

use acadrust::entities::{EntityType, Polyline3D, PolygonMeshEntity, PolygonMeshVertex, Vertex3DPolyline};
use acadrust::tables::Layer;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfVersion, DxfWriter, Handle, Vector3};

fn drawing(vertex_layer: &str) -> (CadDocument, Handle, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.layers.add(Layer::new("WIRES")).unwrap();
    let mut p3 = Polyline3D::new();
    p3.common.layer = "WIRES".into();
    for (x, y, z) in [(0.0, 0.0, 0.0), (10.0, 0.0, 5.0), (10.0, 10.0, 0.0)] {
        let mut v = Vertex3DPolyline::new(Vector3::new(x, y, z));
        v.layer = vertex_layer.into();
        p3.vertices.push(v);
    }
    let h3 = doc.add_entity(EntityType::Polyline3D(p3)).unwrap();
    let mut mesh = PolygonMeshEntity::new();
    mesh.common.layer = "WIRES".into();
    mesh.m_vertex_count = 2;
    mesh.n_vertex_count = 2;
    for (x, y) in [(0.0, 0.0), (0.0, 5.0), (5.0, 0.0), (5.0, 5.0)] {
        let mut v = PolygonMeshVertex::at(Vector3::new(x, y, 1.0));
        v.common.layer = vertex_layer.into();
        mesh.vertices.push(v);
    }
    let hm = doc.add_entity(EntityType::PolygonMesh(mesh)).unwrap();
    (doc, h3, hm)
}

fn via_dwg(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(doc).unwrap())).read().unwrap()
}

/// The layers (8) of the VERTEX records owned by `h` in a DXF's text.
fn vertex_layers(doc: &CadDocument, h: Handle) -> Vec<String> {
    let text = String::from_utf8(DxfWriter::new(doc).keep_heavy_polylines(true).write_to_vec().unwrap()).unwrap();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut records: Vec<Vec<(&str, &str)>> = Vec::new();
    for pair in lines.chunks(2) {
        if pair.len() < 2 {
            break;
        }
        if pair[0] == "0" {
            records.push(Vec::new());
        }
        if let Some(r) = records.last_mut() {
            r.push((pair[0], pair[1]));
        }
    }
    let hex = format!("{:X}", h.value());
    records
        .iter()
        .filter(|r| r[0].1 == "VERTEX" && r.iter().any(|(c, v)| *c == "330" && *v == hex))
        .map(|r| r.iter().find(|(c, _)| *c == "8").map(|(_, v)| v.to_string()).unwrap_or_default())
        .collect()
}

#[test]
fn vertices_read_from_a_dwg_keep_their_layer() {
    let (doc, h3, hm) = drawing("WIRES");
    let read = via_dwg(&doc);
    match read.get_entity(h3).unwrap() {
        EntityType::Polyline3D(p) => assert!(p.vertices.iter().all(|v| v.layer == "WIRES"), "{:?}",
                                             p.vertices.iter().map(|v| &v.layer).collect::<Vec<_>>()),
        _ => panic!("not a 3D polyline"),
    }
    match read.get_entity(hm).unwrap() {
        EntityType::PolygonMesh(m) => assert!(m.vertices.iter().all(|v| v.common.layer == "WIRES"), "{:?}",
                                              m.vertices.iter().map(|v| &v.common.layer).collect::<Vec<_>>()),
        _ => panic!("not a polygon mesh"),
    }
    assert_eq!(vertex_layers(&read, h3), vec!["WIRES"; 3]);
    assert_eq!(vertex_layers(&read, hm), vec!["WIRES"; 4]);
}

#[test]
fn a_vertex_without_a_layer_is_written_on_its_polylines() {
    let (doc, h3, hm) = drawing("");
    assert_eq!(vertex_layers(&doc, h3), vec!["WIRES"; 3]);
    assert_eq!(vertex_layers(&doc, hm), vec!["WIRES"; 4]);
}
