//! A heavy POLYLINE can stay heavy in DXF output, with its VERTEX and SEQEND handles. The DXF writer down-saves a
//! plain 2D polyline to LWPOLYLINE (issue 63), so a DWG read and written back through DXF changed the entity's type,
//! and its sub-records' handles changed on every save (the model kept none for a 2D vertex or any SEQEND). MILI's
//! working DXF must keep what the drawing holds: `DxfWriter::keep_heavy_polylines(true)` writes it as it was read, and
//! the readers keep the VERTEX and SEQEND handles, which both writers write back (MILI's CB-0069; the write gate's
//! three readers flagged the type change on every converted drawing with a plain heavy polyline).
use std::io::Cursor;

use acadrust::entities::{EntityType, Polyline2D, Polyline3D, Vertex2D, Vertex3DPolyline};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter, Handle, Vector3};

fn drawing() -> (CadDocument, Handle, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut pl = Polyline2D::new();
    for (x, y) in [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)] {
        pl.add_vertex(Vertex2D::new(Vector3::new(x, y, 0.0)));
    }
    let h2 = doc.add_entity(EntityType::Polyline2D(pl)).unwrap();
    let mut p3 = Polyline3D::new();
    for (x, y, z) in [(0.0, 0.0, 0.0), (10.0, 0.0, 5.0), (10.0, 10.0, 0.0)] {
        p3.vertices.push(Vertex3DPolyline::new(Vector3::new(x, y, z)));
    }
    let h3 = doc.add_entity(EntityType::Polyline3D(p3)).unwrap();
    (doc, h2, h3)
}

fn dxf(doc: &CadDocument) -> String {
    String::from_utf8(DxfWriter::new(doc).keep_heavy_polylines(true).write_to_vec().unwrap()).unwrap()
}

fn records(text: &str) -> Vec<Vec<(String, String)>> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out: Vec<Vec<(String, String)>> = Vec::new();
    for pair in lines.chunks(2) {
        if pair.len() < 2 {
            break;
        }
        if pair[0] == "0" {
            out.push(Vec::new());
        }
        if let Some(r) = out.last_mut() {
            r.push((pair[0].to_string(), pair[1].to_string()));
        }
    }
    out
}

/// (type of the record with handle `h`, [handles of its VERTEX children], its SEQEND's handle) in a DXF's text.
fn heavy(text: &str, h: Handle) -> (String, Vec<String>, Option<String>) {
    let hex = format!("{:X}", h.value());
    let recs = records(text);
    let kind = recs
        .iter()
        .find(|r| r.iter().any(|(c, v)| c == "5" && v == &hex))
        .map(|r| r[0].1.clone())
        .unwrap_or_default();
    let owned = |r: &Vec<(String, String)>| r.iter().any(|(c, v)| c == "330" && v == &hex);
    let handle = |r: &Vec<(String, String)>| r.iter().find(|(c, _)| c == "5").map(|(_, v)| v.clone());
    let vertices = recs.iter().filter(|r| r[0].1 == "VERTEX" && owned(r)).filter_map(handle).collect();
    let seqend = recs.iter().find(|r| r[0].1 == "SEQEND" && owned(r)).and_then(handle);
    (kind, vertices, seqend)
}

#[test]
fn a_plain_2d_polyline_stays_heavy_when_asked() {
    let (doc, h2, _) = drawing();
    let (kind, vertices, seqend) = heavy(&dxf(&doc), h2);
    assert_eq!(kind, "POLYLINE", "down-saved to LWPOLYLINE");
    assert_eq!(vertices.len(), 4);
    assert!(seqend.is_some());
    let plain = String::from_utf8(DxfWriter::new(&doc).write_to_vec().unwrap()).unwrap();
    assert_eq!(heavy(&plain, h2).0, "LWPOLYLINE", "the default still down-saves (issue 63)");
}

#[test]
fn vertex_and_seqend_handles_survive_dxf_and_dwg() {
    let (doc, h2, h3) = drawing();
    let first = dxf(&doc);
    let read = DxfReader::from_reader(Cursor::new(first.clone().into_bytes())).unwrap().read().unwrap();
    assert_eq!(heavy(&dxf(&read), h2), heavy(&first, h2), "a DXF read and write changed the 2D polyline's records");
    assert_eq!(heavy(&dxf(&read), h3), heavy(&first, h3), "a DXF read and write changed the 3D polyline's records");
    let back = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&read).unwrap())).read().unwrap();
    assert_eq!(heavy(&dxf(&back), h2), heavy(&first, h2), "DXF -> DWG changed the 2D polyline's records");
    assert_eq!(heavy(&dxf(&back), h3), heavy(&first, h3), "DXF -> DWG changed the 3D polyline's records");
}
