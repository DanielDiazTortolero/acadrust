//! An annotative TEXT's scale representation (ACDB_TEXTOBJECTCONTEXTDATA_CLASS) is written to DXF as AutoCAD writes
//! it: its fields (70, 50, 10, 20, 11, 21) straight after the AcDbAnnotScaleObjectContextData groups, with no
//! AcDbTextObjectContextData marker. This crate wrote the marker, and AutoCAD's DXF reader discarded the whole
//! drawing at the first such object ("Missing DXF group code: 1": DWG TrueView 2027 on AutoCAD's own sample read and
//! written here, 2026-10-19; MILI's CB-0038). Its reader looked for the fields under that marker only, so it read
//! AutoCAD's own DXF as zeros; it now reads both forms (MTEXT context data had the same fix before).
use std::io::Cursor;

use acadrust::objects::{ObjectContextData, ObjectContextKind, ObjectType, Scale};
use acadrust::types::{DxfVersion, Vector2};
use acadrust::{CadDocument, DxfReader, DxfWriter, Handle};

fn kind() -> ObjectContextKind {
    ObjectContextKind::Text {
        horizontal_mode: 1,
        rotation: 0.5,
        insertion: Vector2::new(147.8194471194604, 10.32574050131268),
        alignment: Vector2::new(-3.0, 4.0),
    }
}

fn drawing() -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.register_object_context_class("ACDB_TEXTOBJECTCONTEXTDATA_CLASS");
    let scale_h = doc.allocate_handle();
    let mut scale = Scale::new("1:50", 1.0, 50.0);
    scale.handle = scale_h;
    doc.objects.insert(scale_h, ObjectType::Scale(scale));
    let owner_h = doc.allocate_handle();
    let leaf_h = doc.allocate_handle();
    let ctx = ObjectContextData {
        handle: leaf_h,
        owner_handle: owner_h,
        reactors: vec![owner_h],
        xdictionary_handle: None,
        class_version: 4,
        is_default: true,
        scale: scale_h,
        kind: kind(),
    };
    doc.objects.insert(leaf_h, ObjectType::ObjectContextData(ctx));
    (doc, leaf_h)
}

/// The record with handle `h` in a DXF's text: its (code, value) pairs.
fn record(text: &str, h: Handle) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut records: Vec<Vec<(String, String)>> = Vec::new();
    for pair in lines.chunks(2) {
        if pair.len() < 2 {
            break;
        }
        if pair[0] == "0" {
            records.push(Vec::new());
        }
        if let Some(r) = records.last_mut() {
            r.push((pair[0].to_string(), pair[1].to_string()));
        }
    }
    let hex = format!("{:X}", h.value());
    records.into_iter().find(|r| r.iter().any(|(c, v)| c == "5" && *v == hex)).expect("the record")
}

fn read_kind(text: &str, h: Handle) -> ObjectContextKind {
    let doc = DxfReader::from_reader(Cursor::new(text.as_bytes().to_vec())).unwrap().read().unwrap();
    match doc.objects.get(&h) {
        Some(ObjectType::ObjectContextData(c)) => c.kind.clone(),
        other => panic!("not read back as context data: {other:?}"),
    }
}

#[test]
fn written_as_autocad_writes_it() {
    let (doc, h) = drawing();
    let text = String::from_utf8(DxfWriter::new(&doc).write_to_vec().unwrap()).unwrap();
    let rec = record(&text, h);
    let markers: Vec<&str> = rec.iter().filter(|(c, _)| c == "100").map(|(_, v)| v.as_str()).collect();
    assert_eq!(markers, ["AcDbObjectContextData", "AcDbAnnotScaleObjectContextData"]);
    let after: Vec<&str> = rec.iter().skip_while(|(c, _)| c != "340").skip(1).map(|(c, _)| c.as_str()).collect();
    assert_eq!(after, ["70", "50", "10", "20", "11", "21"]);
    assert_eq!(read_kind(&text, h), kind(), "AutoCAD's form read back");
}

#[test]
fn older_output_with_the_marker_still_reads() {
    let (doc, h) = drawing();
    let text = String::from_utf8(DxfWriter::new(&doc).write_to_vec().unwrap()).unwrap();
    // put the marker this crate used to write back in front of the fields (the 70 after the record's 340)
    let hex = format!("{:X}", h.value());
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let start = (0..lines.len() - 1).find(|&i| lines[i].trim() == "5" && lines[i + 1].trim() == hex).expect("record");
    let at = (start..lines.len() - 1)
        .step_by(2)
        .find(|&i| lines[i].trim() == "70" && lines[i - 2].trim() == "340")
        .expect("its 70 after 340");
    lines.splice(at..at, ["100".to_string(), "AcDbTextObjectContextData".to_string()]);
    let legacy = lines.join("\n") + "\n";
    assert!(legacy.contains("AcDbTextObjectContextData"));
    assert_eq!(read_kind(&legacy, h), kind());
}
