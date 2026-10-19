//! A LEADER read from a DWG is the leader AutoCAD's DXF of that drawing lists: its hook line and, from R2010 on, its
//! annotation box are derived, as AutoCAD derives them, and its DWG is written back in the form AutoCAD stores.
//!
//! AutoCAD's own sample drawing (ACadSharp's samples, every release from 2000 to 2018) has a text leader whose last
//! segment is steep: its DWG stores three vertices, its DXF lists four, the third one the start of a hook line 2.5
//! long (ISO-25's arrowhead size) before the last vertex, with the hook flag (75 = 1); from R2010 on its DXF gives the
//! annotation box (40/41) as the annotation MTEXT's extents, which the R2010+ leader record doesn't store. AutoCAD
//! draws a hook when a text leader's last segment is more than 15° off its horizontal direction, one arrowhead long
//! (DIMASZ "also controls the size of hook lines"); LibreDWG derives the flag by the same rule. This crate read the
//! three vertices, no hook and a 2.5/0 box, so its DXF of a DWG drew no hook (and, in ezdxf, no line under the text),
//! and a DWG it wrote from AutoCAD's DXF stored the hook's start as a vertex (MILI's CB-0014).
use std::io::Cursor;

use acadrust::entities::{EntityType, HooklineDirection, Leader, LeaderCreationType, LeaderPathType, MText};
use acadrust::xdata::{ExtendedDataRecord, XDataValue};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter, Handle, Vector3};

/// AutoCAD's sample leader (handle 512) as its DWG stores it, and its MTEXT's stored extents (height, width).
fn stored() -> Vec<Vector3> {
    vec![
        Vector3::new(717.8687154323397, 2.941179455067987, 0.0),
        Vector3::new(729.2681170780972, 48.538786038098586, 0.0),
        Vector3::new(740.3987500252849, 14.071812402255562, 0.0),
    ]
}
const EXTENTS: (f64, f64) = (1.844821309997273, 16.37400410337938);

/// ...and as AutoCAD's DXF lists it: the hook's start, 2.5 before the last vertex, inserted before it.
fn listed() -> Vec<Vector3> {
    let mut v = stored();
    v.insert(2, Vector3::new(737.8987500252849, 14.071812402255562, 0.0));
    v
}

fn drawing(version: DxfVersion, vertices: Vec<Vector3>, hook: bool, edit: impl FnOnce(&mut Leader)) -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(version);
    if let Some(style) = doc.dim_styles.get_mut("Standard") {
        style.dimasz = 2.5;
        style.dimscale = 1.0;
    }
    let mut text = MText::with_value("Sample annotation", Vector3::new(741.0237500252849, 14.696812402255562, 0.0));
    text.height = 1.4249252057197062;
    text.extents_height = EXTENTS.0;
    text.extents_width = EXTENTS.1;
    let ht = doc.add_entity(EntityType::MText(Box::new(text))).unwrap();
    let mut leader = Leader::from_vertices(vertices);
    leader.creation_type = LeaderCreationType::WithText;
    leader.annotation_handle = ht;
    leader.hookline_enabled = hook;
    leader.text_height = EXTENTS.0;
    leader.text_width = EXTENTS.1;
    edit(&mut leader);
    let hl = doc.add_entity(EntityType::Leader(leader)).unwrap();
    (doc, hl)
}

fn via_dwg(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(doc).unwrap())).read().unwrap()
}

fn via_dxf(doc: &CadDocument) -> CadDocument {
    let text = DxfWriter::new(doc).write_to_vec().unwrap();
    DxfReader::from_reader(Cursor::new(text)).unwrap().read().unwrap()
}

fn leader(doc: &CadDocument, h: Handle) -> Leader {
    match doc.get_entity(h).unwrap() {
        EntityType::Leader(l) => l.clone(),
        other => panic!("not a leader: {}", other.as_entity().entity_type()),
    }
}

fn close(a: &[Vector3], b: &[Vector3]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| p.distance(q) <= 1e-9)
}

const VERSIONS: [DxfVersion; 5] = [DxfVersion::AC1015, DxfVersion::AC1018, DxfVersion::AC1021, DxfVersion::AC1024, DxfVersion::AC1032];

#[test]
fn a_dwg_leader_reads_as_autocads_dxf_lists_it() {
    for version in VERSIONS {
        // the DWG form: three vertices, no flag (nothing to leave out on writing)
        let (doc, h) = drawing(version, stored(), false, |_| {});
        let read = leader(&via_dwg(&doc), h);
        assert!(close(&read.vertices, &listed()), "{version:?}: {:?}", read.vertices);
        assert!(read.hookline_enabled, "{version:?}: the hook flag");
        assert_eq!((read.text_height, read.text_width), EXTENTS, "{version:?}: the annotation box");
    }
}

#[test]
fn the_dwg_stores_the_hook_as_autocad_does() {
    for version in VERSIONS {
        // AutoCAD's DXF form, through DXF and DWG: the DWG leaves the hook's start out (a stored vertex would end the
        // leader along its horizontal direction, so its reading would find no hook and lose the flag), and reading
        // it gives AutoCAD's DXF form back
        let (doc, h) = drawing(version, listed(), true, |_| {});
        let back = leader(&via_dwg(&via_dxf(&doc)), h);
        assert!(close(&back.vertices, &listed()), "{version:?}: {:?}", back.vertices);
        assert!(back.hookline_enabled, "{version:?}: the hook flag");
        assert_eq!((back.text_height, back.text_width), EXTENTS, "{version:?}");
    }
}

#[test]
fn a_box_from_a_dxf_reaches_a_2010_dwg_through_its_annotation() {
    // AutoCAD's DXF gives no MTEXT extents (42/43 are read-only); an R2010+ DWG keeps the leader's box only there
    let (mut doc, h) = drawing(DxfVersion::AC1032, listed(), true, |_| {});
    let ht = leader(&doc, h).annotation_handle;
    if let Some(EntityType::MText(m)) = doc.get_entity_mut(ht) {
        m.extents_height = 0.0;
        m.extents_width = 0.0;
    }
    let back = leader(&via_dwg(&doc), h);
    assert_eq!((back.text_height, back.text_width), EXTENTS);
}

#[test]
fn a_hook_autocad_would_draw_otherwise_stays_a_vertex() {
    // a landing 3.0 long where AutoCAD draws 2.5: stored as a vertex, so the leader looks the same
    let mut v = stored();
    v.insert(2, Vector3::new(740.3987500252849 - 3.0, 14.071812402255562, 0.0));
    let (doc, h) = drawing(DxfVersion::AC1032, v.clone(), true, |_| {});
    let back = leader(&via_dwg(&doc), h);
    assert!(close(&back.vertices, &v), "{:?}", back.vertices);
}

#[test]
fn no_hook_off_the_rule() {
    // a last segment within 15° of horizontal, a spline path, a block annotation: no hook
    let flat = vec![Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 10.0, 0.0), Vector3::new(20.0, 12.0, 0.0)];
    let (doc, h) = drawing(DxfVersion::AC1032, flat.clone(), false, |_| {});
    let back = leader(&via_dwg(&doc), h);
    assert!(close(&back.vertices, &flat) && !back.hookline_enabled);
    let (doc, h) = drawing(DxfVersion::AC1032, stored(), false, |l| l.path_type = LeaderPathType::Spline);
    assert!(close(&leader(&via_dwg(&doc), h).vertices, &stored()));
    let (doc, h) = drawing(DxfVersion::AC1032, stored(), false, |l| l.creation_type = LeaderCreationType::WithBlock);
    assert!(close(&leader(&via_dwg(&doc), h).vertices, &stored()));
}

#[test]
fn the_hook_follows_the_style_its_overrides_and_its_direction() {
    let last = *stored().last().unwrap();
    // DIMSCALE 2 on the style, DIMASZ 4 overridden on the leader: 8 long
    let (mut doc, h) = drawing(DxfVersion::AC1032, stored(), false, |l| {
        let mut r = ExtendedDataRecord::new("ACAD");
        r.values = vec![
            XDataValue::String("DSTYLE".into()),
            XDataValue::ControlString("{".into()),
            XDataValue::Integer16(41),
            XDataValue::Real(4.0),
            XDataValue::ControlString("}".into()),
        ];
        l.common.extended_data.add_record(r);
    });
    doc.dim_styles.get_mut("Standard").unwrap().dimscale = 2.0;
    let back = leader(&via_dwg(&doc), h);
    assert!(back.vertices[2].distance(&(last - Vector3::new(8.0, 0.0, 0.0))) <= 1e-9, "{:?}", back.vertices);
    // the hook on the far side of the last vertex (74 = 1)
    let (doc, h) = drawing(DxfVersion::AC1032, stored(), false, |l| l.hookline_direction = HooklineDirection::Same);
    let back = leader(&via_dwg(&doc), h);
    assert!(back.vertices[2].distance(&(last + Vector3::new(2.5, 0.0, 0.0))) <= 1e-9, "{:?}", back.vertices);
}

#[test]
fn the_dxf_lists_what_autocads_does() {
    let (doc, h) = drawing(DxfVersion::AC1032, stored(), false, |_| {});
    let text = String::from_utf8(DxfWriter::new(&via_dwg(&doc)).write_to_vec().unwrap()).unwrap();
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
    let rec = records.iter().find(|r| r.iter().any(|(c, v)| *c == "5" && *v == hex)).expect("the leader");
    let get = |c: &str| rec.iter().find(|(k, _)| *k == c).map(|(_, v)| *v);
    assert_eq!(get("76"), Some("4"));
    assert_eq!(get("75"), Some("1"));
    let xs: Vec<f64> = rec.iter().filter(|(c, _)| *c == "10").map(|(_, v)| v.parse().unwrap()).collect();
    assert!((xs[2] - 737.8987500252849).abs() <= 1e-9, "the hook's start: {xs:?}");
}

#[test]
fn an_annotative_leader_gets_no_derived_hook() {
    // its hook scales with the annotation scale, which isn't modelled here: no vertex is made up, the flag is set
    let (mut doc, h) = drawing(DxfVersion::AC1032, stored(), false, |_| {});
    doc.dim_styles.get_mut("Standard").unwrap().annotative = true;
    let read = via_dwg(&doc);
    assert!(read.dim_styles.get("Standard").unwrap().annotative, "the style's annotative flag didn't survive");
    let back = leader(&read, h);
    assert!(close(&back.vertices, &stored()), "{:?}", back.vertices);
    assert!(back.hookline_enabled);
}
