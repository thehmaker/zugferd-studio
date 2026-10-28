// src-tauri/src/zugferd_stripper.rs
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashSet;
use std::path::Path;

const ZUGFERD_FILENAMES: [&str; 3] = [
    "factur-x.xml",
    "zugferd-invoice.xml",
    "xrechnung.xml",
];

pub fn strip_zugferd_from_pdf<P: AsRef<Path>, Q: AsRef<Path>>(
    source_pdf: P,
    target_pdf: Q,
) -> Result<bool, String> {
    let mut doc = Document::load(source_pdf)
        .map_err(|e| format!("Fehler beim Laden des PDFs: {e}"))?;

    let mut filespec_ids: HashSet<ObjectId> = HashSet::new();
    let mut stream_ids: HashSet<ObjectId> = HashSet::new();

    // 1. Alle ZUGFeRD-relevanten Filespecs und deren Streams identifizieren
    for (id, obj) in &doc.objects {
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
                filespec_ids.insert(*id);

                if let Ok(ef_dict) = dict.get(b"EF").and_then(|ef| ef.as_dict()) {
                    let stream_ref = ef_dict.get(b"F").or_else(|_| ef_dict.get(b"UF"));
                    if let Ok(s_id) = stream_ref.and_then(|r| r.as_reference()) {
                        stream_ids.insert(s_id);
                    }
                }
            }
        }
    }

    if filespec_ids.is_empty() {
        return Ok(false);
    }

    // 2. Dokument-Katalog ohne konkurrierende Borrows bereinigen
    clean_catalog(&mut doc, &filespec_ids)?;

    // 3. XMP-Metadaten-Stream säubern
    clean_xmp_metadata(&mut doc);

    // 4. Nicht mehr referenzierte Objekte aus doc.objects entfernen
    for id in &filespec_ids {
        doc.objects.remove(id);
    }
    for id in &stream_ids {
        doc.objects.remove(id);
    }

    // 5. Gesäubertes PDF speichern
    doc.save(target_pdf)
        .map_err(|e| format!("Fehler beim Speichern des bereinigten PDFs: {e}"))?;

    Ok(true)
}

fn clean_catalog(doc: &mut Document, filespec_ids: &HashSet<ObjectId>) -> Result<(), String> {
    let catalog_ref = doc
        .trailer
        .get(b"Root")
        .and_then(|r| r.as_reference())
        .map_err(|_| "Kein Root-Katalog gefunden".to_string())?;

    // Scope 1: Nur catalog mutabel ausleihen, /AF säubern und IDs extrahieren
    let (names_obj_id, inline_names_dict) = {
        let catalog = doc
            .get_object_mut(catalog_ref)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|_| "Root-Objekt ist kein Dictionary".to_string())?;

        // /AF säubern
        if let Ok(af_array) = catalog.get_mut(b"AF").and_then(|a| a.as_array_mut()) {
            af_array.retain(|item| {
                if let Ok(id) = item.as_reference() {
                    !filespec_ids.contains(&id)
                } else {
                    true
                }
            });
        }
        if let Ok(af_array) = catalog.get(b"AF").and_then(|a| a.as_array()) {
            if af_array.is_empty() {
                catalog.remove(b"AF");
            }
        }

        // Referenzen für /Names ermitteln
        let names_ref = catalog.get(b"Names").and_then(|n| n.as_reference()).ok();
        let inline_names = if names_ref.is_none() {
            catalog.get(b"Names").and_then(|n| n.as_dict()).cloned().ok()
        } else {
            None
        };

        (names_ref, inline_names)
    }; // <-- Der mutable borrow von catalog endet exakt HIER. doc ist wieder frei!

    // Scope 2: /EmbeddedFiles im Namensbaum bearbeiten
    if let Some(names_id) = names_obj_id {
        clean_names_dict_by_id(doc, names_id, filespec_ids);
    } else if let Some(mut names_dict) = inline_names_dict {
        clean_names_dict_inline(doc, &mut names_dict, filespec_ids);
        if let Ok(catalog) = doc.get_object_mut(catalog_ref).and_then(|o| o.as_dict_mut()) {
            catalog.set("Names", Object::Dictionary(names_dict));
        }
    }

    Ok(())
}

fn clean_names_dict_by_id(doc: &mut Document, names_id: ObjectId, filespec_ids: &HashSet<ObjectId>) {
    // 1. Prüfen, ob EmbeddedFiles eine ID oder ein Inline-Dict ist
    let ef_target = {
        let names_dict = match doc.get_object(names_id).and_then(|o| o.as_dict()) {
            Ok(d) => d,
            Err(_) => return,
        };
        let ef_ref = names_dict.get(b"EmbeddedFiles").and_then(|e| e.as_reference()).ok();
        let ef_inline = if ef_ref.is_none() {
            names_dict.get(b"EmbeddedFiles").and_then(|e| e.as_dict()).cloned().ok()
        } else {
            None
        };
        (ef_ref, ef_inline)
    }; // Lesender Borrow von doc beendet

    // 2. Jetzt mutieren
    if let Some(ef_id) = ef_target.0 {
        if let Ok(ef_dict) = doc.get_object_mut(ef_id).and_then(|o| o.as_dict_mut()) {
            filter_names_array(ef_dict, filespec_ids);
        }
    } else if let Some(mut ef_dict) = ef_target.1 {
        filter_names_array(&mut ef_dict, filespec_ids);
        if let Ok(names_dict) = doc.get_object_mut(names_id).and_then(|o| o.as_dict_mut()) {
            names_dict.set("EmbeddedFiles", Object::Dictionary(ef_dict));
        }
    }
}

fn clean_names_dict_inline(
    doc: &mut Document,
    names_dict: &mut Dictionary,
    filespec_ids: &HashSet<ObjectId>,
) {
    let ef_ref = names_dict.get(b"EmbeddedFiles").and_then(|e| e.as_reference()).ok();
    if let Some(ef_id) = ef_ref {
        if let Ok(ef_dict) = doc.get_object_mut(ef_id).and_then(|o| o.as_dict_mut()) {
            filter_names_array(ef_dict, filespec_ids);
        }
    } else if let Ok(ef_dict) = names_dict.get_mut(b"EmbeddedFiles").and_then(|e| e.as_dict_mut()) {
        filter_names_array(ef_dict, filespec_ids);
    }
}

fn filter_names_array(ef_dict: &mut Dictionary, filespec_ids: &HashSet<ObjectId>) {
    if let Ok(names_arr) = ef_dict.get_mut(b"Names").and_then(|n| n.as_array_mut()) {
        let mut new_arr = Vec::new();
        let mut i = 0;
        while i + 1 < names_arr.len() {
            let val = &names_arr[i + 1];
            let should_remove = if let Ok(ref_id) = val.as_reference() {
                filespec_ids.contains(&ref_id)
            } else {
                false
            };

            if !should_remove {
                new_arr.push(names_arr[i].clone());
                new_arr.push(names_arr[i + 1].clone());
            }
            i += 2;
        }
        *names_arr = new_arr;
    }
}

fn clean_xmp_metadata(doc: &mut Document) {
    for (_id, obj) in doc.objects.iter_mut() {
        if let Ok(stream) = obj.as_stream_mut() {
            let is_metadata = stream
                .dict
                .get(b"Type")
                .and_then(|t| t.as_name())
                .map(|n| n == b"Metadata")
                .unwrap_or(false);

            if !is_metadata {
                continue;
            }

            if let Ok(content_bytes) = stream.decompressed_content() {
                let content_str = String::from_utf8_lossy(&content_bytes);

                if content_str.contains("CrossIndustryDocument") || content_str.contains("urn:factur-x") {
                    let mut cleaned_str = content_str.to_string();
                    if let Some(start) = cleaned_str.find("<rdf:Description") {
                        if let Some(end) = cleaned_str[start..].find("</rdf:Description>") {
                            let block = &cleaned_str[start..start + end + 18];
                            if block.contains("factur-x") || block.contains("CrossIndustryDocument") {
                                cleaned_str.replace_range(start..start + end + 18, "");
                            }
                        }
                    }

                    stream.set_content(cleaned_str.into_bytes());
                }
            }
        }
    }
}