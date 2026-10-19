//! An MTEXT's background mask is written to DXF as AutoCAD writes it: the fill flags (90), the colour as an index
//! (63) always, its true colour (421) when it has one, the box scale (45) and the transparency (441) always, in that
//! order, and not at all into a 2000 DXF (masks came with 2004; AutoCAD's own R2000 DXF of its sample has none). This
//! crate wrote 90, 45 and then 421 or 63, and 441 only when not 0, and AutoCAD's DXF reader stopped at the first
//! masked MTEXT and discarded the whole drawing: "Could not parse embeddedObject marker" in a 2018 DXF, "Xdata wasn't
//! read" in a 2004 one (DWG TrueView 2027 on AutoCAD's own sample read and written here, 2026-10-19; MILI's CB-0013).
use std::io::Cursor;

use acadrust::entities::{EntityType, MText};
use acadrust::types::Color;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter, Handle, Vector3};

fn drawing(version: DxfVersion, color: Color) -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(version);
    let mut m = MText::with_value("\\A1;7.0711", Vector3::new(37.65, -119.69, 0.0));
    m.height = 0.18;
    m.background_fill_flags = 1;
    m.background_color = color;
    m.background_scale = 1.25;
    m.background_transparency = 0;
    let h = doc.add_entity(EntityType::MText(Box::new(m))).unwrap();
    (doc, h)
}

fn dxf(doc: &CadDocument) -> String {
    String::from_utf8(DxfWriter::new(doc).write_to_vec().unwrap()).unwrap()
}

/// The MTEXT record `h`'s (code, value) pairs from code 90 on, in the order written.
fn fill_codes(text: &str, h: Handle) -> Vec<(String, String)> {
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
    let rec = records.iter().find(|r| r.iter().any(|(c, v)| *c == "5" && *v == hex)).expect("the MTEXT");
    rec.iter()
        .skip_while(|(c, _)| *c != "90")
        .filter(|(c, _)| ["90", "63", "421", "431", "45", "441"].contains(c))
        .map(|(c, v)| (c.to_string(), v.to_string()))
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(c, v)| (c.to_string(), v.to_string())).collect()
}

#[test]
fn a_true_colour_mask_is_written_as_autocad_writes_it() {
    let green = Color::from_rgb(54, 117, 66);
    for version in [DxfVersion::AC1018, DxfVersion::AC1024, DxfVersion::AC1032] {
        let (doc, h) = drawing(version, green);
        let aci = green.approximate_index().to_string();
        assert_eq!(fill_codes(&dxf(&doc), h),
                   pairs(&[("90", "1"), ("63", &aci), ("421", "3568962"), ("45", "1.25"), ("441", "0")]),
                   "{version:?}");
    }
}

#[test]
fn an_index_colour_mask_has_no_true_colour() {
    let (doc, h) = drawing(DxfVersion::AC1032, Color::from_index(3));
    assert_eq!(fill_codes(&dxf(&doc), h), pairs(&[("90", "1"), ("63", "3"), ("45", "1.25"), ("441", "0")]));
}

#[test]
fn a_2000_dxf_has_no_mask() {
    let (doc, h) = drawing(DxfVersion::AC1015, Color::from_rgb(54, 117, 66));
    assert_eq!(fill_codes(&dxf(&doc), h), Vec::<(String, String)>::new());
}

#[test]
fn the_mask_survives_dxf_and_dwg() {
    let green = Color::from_rgb(54, 117, 66);
    let (doc, h) = drawing(DxfVersion::AC1032, green);
    let via_dxf = DxfReader::from_reader(Cursor::new(dxf(&doc).into_bytes())).unwrap().read().unwrap();
    let via_dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&via_dxf).unwrap())).read().unwrap();
    for read in [&via_dxf, &via_dwg] {
        match read.get_entity(h).unwrap() {
            EntityType::MText(m) => {
                assert_eq!(m.background_fill_flags, 1);
                assert_eq!(m.background_color, green);
                assert_eq!(m.background_scale, 1.25);
            }
            other => panic!("not an MTEXT: {}", other.as_entity().entity_type()),
        }
    }
}
