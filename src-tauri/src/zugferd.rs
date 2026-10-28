// src-tauri/src/zugferd.rs
use lopdf::{Document, Object, ObjectId};
use serde::{Deserialize, Serialize};
use std::path::Path;

// Unseren neuen Parser importieren
use crate::zugferd_parser::{parse_cii_xml, ZugferdInvoice};

const ZUGFERD_FILENAMES: [&str; 3] = [
    "factur-x.xml",
    "zugferd-invoice.xml",
    "xrechnung.xml",
];

#[derive(Debug, Serialize, Deserialize)]
pub struct ZugferdDetection {
    pub has_zugferd: bool,
    pub filename: Option<String>,
    pub raw_xml: Option<String>,
    pub xml_size_bytes: usize,
    // NEU: Die typisierten Rechnungsdaten
    pub invoice: Option<ZugferdInvoice>,
}

pub fn inspect_pdf<P: AsRef<Path>>(path: P) -> Result<ZugferdDetection, String> {
    let doc = Document::load(path).map_err(|e| format!("PDF konnte nicht geladen werden: {e}"))?;

    for (_id, obj) in &doc.objects {
        if let Ok(dict) = obj.as_dict() {
            let is_filespec = dict
                .get(b"Type")
                .and_then(|t| t.as_name())
                .map(|name| name == b"Filespec")
                .unwrap_or(false)
                || dict.has(b"EF");

            if !is_filespec {
                continue;
            }

            let filename_obj = dict.get(b"UF").or_else(|_| dict.get(b"F"));
            let filename = match filename_obj {
                Ok(Object::String(bytes, _)) => String::from_utf8_lossy(bytes).to_string(),
                _ => continue,
            };

            let filename_lower = filename.to_lowercase();
            if ZUGFERD_FILENAMES.iter().any(|target| filename_lower.contains(target)) {
                if let Ok(ef_dict) = dict.get(b"EF").and_then(|ef| ef.as_dict()) {
                    let stream_ref = ef_dict.get(b"F").or_else(|_| ef_dict.get(b"UF"));

                    if let Ok(stream_id) = stream_ref.and_then(|r| r.as_reference()) {
                        if let Some(xml_content) = extract_stream_content(&doc, stream_id) {
                            // NEU: XML sofort typisiert mit quick-xml parsen
                            let invoice = parse_cii_xml(&xml_content).ok();

                            return Ok(ZugferdDetection {
                                has_zugferd: true,
                                filename: Some(filename),
                                xml_size_bytes: xml_content.len(),
                                raw_xml: Some(xml_content),
                                invoice,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(ZugferdDetection {
        has_zugferd: false,
        filename: None,
        raw_xml: None,
        xml_size_bytes: 0,
        invoice: None,
    })
}

fn extract_stream_content(doc: &Document, stream_id: ObjectId) -> Option<String> {
    let stream_obj = doc.get_object(stream_id).ok()?;
    let stream = stream_obj.as_stream().ok()?;

    let decompressed = stream.decompressed_content().ok()?;
    let content = String::from_utf8_lossy(&decompressed).to_string();

    if content.contains("CrossIndustryInvoice")
        || content.contains("rsm:CrossIndustryInvoice")
        || content.contains("CrossIndustryDocument")
    {
        Some(content)
    } else {
        None
    }
}