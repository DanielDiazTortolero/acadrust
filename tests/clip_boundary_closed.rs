//! A polygonal clip boundary (WIPEOUT, IMAGE) is closed in a DXF and open in a DWG, as AutoCAD writes them. AutoCAD's
//! own sample drawing (ACadSharp's samples, every release from 2000 to 2018) has a four-corner wipeout: its DXF lists
//! five vertices, the fifth one the first again (91 5), its DWG stores the four (LibreDWG reads it the same way).
//! This crate wrote whatever the model held, so a DWG read and written to DXF had an open boundary, which AutoCAD may
//! not take (ezdxf closes one before writing it: "otherwise AutoCAD crashes"), and a DXF written to DWG stored the
//! closing vertex. Now the DXF writer closes an open polygon and the DWG writer stores a closed one open, whichever
//! form the model holds (MILI's CB-0015).
use std::io::Cursor;

use acadrust::entities::{ClipBoundary, EntityType, RasterImage, Wipeout};
use acadrust::types::Vector2;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter, Handle, Vector3};

/// AutoCAD's sample wipeout boundary (handle 44D), as its DWG stores it.
fn corners() -> Vec<Vector2> {
    vec![
        Vector2::new(-0.5, 0.5),
        Vector2::new(0.3260511008800897, 0.49206350233605894),
        Vector2::new(0.2505944934505404, -0.21031745948902658),
        Vector2::new(-0.47617156637515023, -0.5),
    ]
}

fn closed() -> Vec<Vector2> {
    let mut v = corners();
    v.push(v[0]);
    v
}

fn drawing(boundary: Vec<Vector2>, version: DxfVersion) -> (CadDocument, Handle, Handle) {
    let mut doc = CadDocument::with_version(version);
    let mut w = Wipeout::new();
    w.set_polygon(&boundary);
    let hw = doc.add_entity(EntityType::Wipeout(w)).unwrap();
    let mut image = RasterImage::new("photo.png", Vector3::new(10.0, 0.0, 0.0), 640.0, 480.0);
    image.clip_boundary = ClipBoundary::polygonal(boundary);
    image.clipping_enabled = true;
    let hi = doc.add_entity(EntityType::RasterImage(image)).unwrap();
    (doc, hw, hi)
}

fn dxf(doc: &CadDocument) -> String {
    String::from_utf8(DxfWriter::new(doc).write_to_vec().unwrap()).unwrap()
}

/// The (91 count, [(14, 24) vertices]) of the record with handle `h` in a DXF's text.
fn boundary_in(text: &str, h: Handle) -> (usize, Vec<(f64, f64)>) {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let hex = format!("{:X}", h.value());
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
    let rec = records.iter().find(|r| r.iter().any(|(c, v)| *c == "5" && *v == hex)).expect("record written");
    let count = rec.iter().find(|(c, _)| *c == "91").map(|(_, v)| v.parse().unwrap()).unwrap_or(0);
    let xs: Vec<f64> = rec.iter().filter(|(c, _)| *c == "14").map(|(_, v)| v.parse().unwrap()).collect();
    let ys: Vec<f64> = rec.iter().filter(|(c, _)| *c == "24").map(|(_, v)| v.parse().unwrap()).collect();
    (count, xs.into_iter().zip(ys).collect())
}

fn dwg_round_trip(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(doc).unwrap())).read().unwrap()
}

fn wipeout(doc: &CadDocument, h: Handle) -> Vec<Vector2> {
    match doc.get_entity(h).unwrap() {
        EntityType::Wipeout(w) => w.clip_boundary_vertices.clone(),
        other => panic!("not a wipeout: {}", other.as_entity().entity_type()),
    }
}

fn image(doc: &CadDocument, h: Handle) -> Vec<Vector2> {
    match doc.get_entity(h).unwrap() {
        EntityType::RasterImage(i) => i.clip_boundary.vertices.clone(),
        other => panic!("not an image: {}", other.as_entity().entity_type()),
    }
}

#[test]
fn dxf_closes_an_open_polygon() {
    let (doc, hw, hi) = drawing(corners(), DxfVersion::AC1032);
    let text = dxf(&doc);
    for h in [hw, hi] {
        let (count, verts) = boundary_in(&text, h);
        assert_eq!((count, verts.len()), (5, 5), "AutoCAD's DXF closes a polygonal boundary");
        assert_eq!(verts[4], verts[0], "the fifth vertex is the first");
    }
}

#[test]
fn dxf_keeps_a_closed_polygon_as_it_is() {
    let (doc, hw, hi) = drawing(closed(), DxfVersion::AC1032);
    let text = dxf(&doc);
    for h in [hw, hi] {
        assert_eq!(boundary_in(&text, h).0, 5, "closed twice");
    }
}

#[test]
fn a_rectangle_stays_two_corners() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let h = doc.add_entity(EntityType::Wipeout(Wipeout::rectangular(Vector3::ZERO, 4.0, 3.0))).unwrap();
    assert_eq!(boundary_in(&dxf(&doc), h).0, 2);
}

#[test]
fn dwg_stores_a_closed_polygon_open() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1018, DxfVersion::AC1024, DxfVersion::AC1032] {
        let (doc, hw, hi) = drawing(closed(), version);
        let read = dwg_round_trip(&doc);
        assert_eq!(wipeout(&read, hw), corners(), "{version:?}: the wipeout's DWG keeps the closing vertex");
        assert_eq!(image(&read, hi), corners(), "{version:?}: the image's DWG keeps the closing vertex");
    }
}

#[test]
fn dwg_keeps_an_open_polygon() {
    let (doc, hw, hi) = drawing(corners(), DxfVersion::AC1032);
    let read = dwg_round_trip(&doc);
    assert_eq!(wipeout(&read, hw), corners());
    assert_eq!(image(&read, hi), corners());
}

#[test]
fn autocads_dxf_form_survives_dxf_to_dwg_to_dxf() {
    // AutoCAD's DXF (closed) -> this crate's DXF reading -> DWG (open) -> its reading -> DXF (closed again)
    let (doc, hw, hi) = drawing(closed(), DxfVersion::AC1032);
    let first = dxf(&doc);
    let read = DxfReader::from_reader(Cursor::new(first.clone().into_bytes())).unwrap().read().unwrap();
    let again = dxf(&dwg_round_trip(&read));
    for h in [hw, hi] {
        assert_eq!(boundary_in(&again, h), boundary_in(&first, h));
    }
}
