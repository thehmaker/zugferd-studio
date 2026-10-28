use flate2::write::ZlibEncoder;
use flate2::Compression;
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use std::io::Write;
use std::path::Path;

const ZUGFERD_FILENAMES: [&str; 3] = [
    "factur-x.xml",
    "zugferd-invoice.xml",
    "xrechnung.xml",
];

pub fn embed_or_replace_xml<P: AsRef<Path>, Q: AsRef<Path>>(
    source_pdf: P,
    target_pdf: Q,
    xml_content: &str,
    profile_name: &str,
) -> Result<(), String> {
    let mut doc = Document::load(source_pdf)
        .map_err(|e| format!("Fehler beim Laden des Original-PDFs: {e}"))?;

    // 1. XML mit Zlib komprimieren
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(xml_content.as_bytes())
        .map_err(|e| format!("Komprimierungsfehler: {e}"))?;
    let compressed_bytes = encoder
        .finish()
        .map_err(|e| format!("Fehler beim Fertigstellen des Streams: {e}"))?;

    // 2. Stream ersetzen oder neu anhängen
    let existing_stream_id = find_existing_xml_stream(&doc);

    if let Some(stream_id) = existing_stream_id {
        update_existing_stream(&mut doc, stream_id, compressed_bytes, xml_content.len())?;
    } else {
        attach_new_zugferd(&mut doc, compressed_bytes, xml_content.len())?;
    }

    // 3. XMP-Metadaten für PDF/A-3 und ZUGFeRD setzen
    inject_zugferd_xmp_metadata(&mut doc, profile_name)?;

    // 4. Speichern
    doc.save(target_pdf)
        .map_err(|e| format!("Fehler beim Speichern der PDF-Datei: {e}"))?;

    Ok(())
}

fn inject_zugferd_xmp_metadata(doc: &mut Document, profile: &str) -> Result<(), String> {
    let xmp_template = format!(
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about="" xmlns:pdfaExtension="http://www.aiim.org/pdfa/ns/extension/" xmlns:pdfaProperty="http://www.aiim.org/pdfa/ns/property#">
      <pdfaExtension:schemas>
        <rdf:Bag>
          <rdf:li rdf:parseType="Resource">
            <pdfaProperty:name>ConformanceLevel</pdfaProperty:name>
            <pdfaProperty:valueType>Text</pdfaProperty:valueType>
            <pdfaProperty:description>The conformance level of the Factur-X/ZUGFeRD invoice</pdfaProperty:description>
          </rdf:li>
        </rdf:Bag>
      </pdfaExtension:schemas>
    </rdf:Description>
    <rdf:Description rdf:about="" xmlns:zf="urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#">
      <zf:DocumentType>INVOICE</zf:DocumentType>
      <zf:DocumentFileName>factur-x.xml</zf:DocumentFileName>
      <zf:Version>1.0</zf:Version>
      <zf:ConformanceLevel>{}</zf:ConformanceLevel>
    </rdf:Description>
    <rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">
      <pdfaid:part>3</pdfaid:part>
      <pdfaid:conformance>B</pdfaid:conformance>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#,
        profile.to_uppercase()
    );

    // Prüfen, ob bereits ein Metadata-Objekt im Katalog existiert
    let catalog_ref = doc
        .trailer
        .get(b"Root")
        .and_then(|r| r.as_reference())
        .map_err(|_| "Kein Root-Katalog vorhanden".to_string())?;

    let mut metadata_dict = Dictionary::new();
    metadata_dict.set("Type", Object::Name(b"Metadata".to_vec()));
    metadata_dict.set("Subtype", Object::Name(b"XML".to_vec()));

    let stream = Stream::new(metadata_dict, xmp_template.into_bytes());
    let metadata_obj_id = doc.add_object(Object::Stream(stream));

    let catalog = doc
        .get_object_mut(catalog_ref)
        .and_then(|obj| obj.as_dict_mut())
        .map_err(|_| "Root-Objekt ist kein Dictionary".to_string())?;

    catalog.set("Metadata", Object::Reference(metadata_obj_id));

    Ok(())
}

fn find_existing_xml_stream(doc: &Document) -> Option<ObjectId> {
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
                        return Some(stream_id);
                    }
                }
            }
        }
    }
    None
}

fn update_existing_stream(
    doc: &mut Document,
    stream_id: ObjectId,
    compressed_data: Vec<u8>,
    uncompressed_len: usize,
) -> Result<(), String> {
    let stream_obj = doc
        .get_object_mut(stream_id)
        .map_err(|e| format!("Stream-Objekt {stream_id:?} nicht gefunden: {e}"))?;

    let stream = stream_obj
        .as_stream_mut()
        .map_err(|_| "Zielobjekt ist kein Stream".to_string())?;

    stream.set_content(compressed_data);
    stream
        .dict
        .set("Filter", Object::Name(b"FlateDecode".to_vec()));
    stream
        .dict
        .set("Subtype", Object::Name(b"text#2Fxml".to_vec()));
    stream
        .dict
        .set("Type", Object::Name(b"EmbeddedFile".to_vec()));

    let mut params = Dictionary::new();
    params.set("Size", Object::Integer(uncompressed_len as i64));
    stream.dict.set("Params", Object::Dictionary(params));

    Ok(())
}

fn attach_new_zugferd(
    doc: &mut Document,
    compressed_data: Vec<u8>,
    uncompressed_len: usize,
) -> Result<(), String> {
    // 1. EmbeddedFile Stream anlegen
    let mut stream_dict = Dictionary::new();
    stream_dict.set("Type", Object::Name(b"EmbeddedFile".to_vec()));
    stream_dict.set("Subtype", Object::Name(b"text#2Fxml".to_vec()));
    stream_dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));

    let mut params = Dictionary::new();
    params.set("Size", Object::Integer(uncompressed_len as i64));
    stream_dict.set("Params", Object::Dictionary(params));

    let stream_obj = Object::Stream(Stream::new(stream_dict, compressed_data));
    let stream_id = doc.add_object(stream_obj);

    // 2. Filespec Dictionary anlegen
    let mut filespec = Dictionary::new();
    filespec.set("Type", Object::Name(b"Filespec".to_vec()));
    filespec.set(
        "F",
        Object::String(b"factur-x.xml".to_vec(), StringFormat::Literal),
    );
    filespec.set(
        "UF",
        Object::String(b"factur-x.xml".to_vec(), StringFormat::Literal),
    );
    filespec.set(
        "AFRelationship",
        Object::Name(b"Alternative".to_vec()),
    );

    let mut ef_dict = Dictionary::new();
    ef_dict.set("F", Object::Reference(stream_id));
    ef_dict.set("UF", Object::Reference(stream_id));
    filespec.set("EF", Object::Dictionary(ef_dict));

    let filespec_id = doc.add_object(Object::Dictionary(filespec));

    // 3. Im Dokument-Katalog (Catalog / Root) registrieren
    let catalog_ref = doc
        .trailer
        .get(b"Root")
        .and_then(|r| r.as_reference())
        .map_err(|_| "Dokument besitzt keinen Root-Katalog".to_string())?;

    let catalog = doc
        .get_object_mut(catalog_ref)
        .and_then(|obj| obj.as_dict_mut())
        .map_err(|_| "Root-Objekt ist kein Dictionary".to_string())?;

    // /AF Array (Associated Files nach PDF/A-3)
    if let Ok(af_array) = catalog.get_mut(b"AF").and_then(|a| a.as_array_mut()) {
        af_array.push(Object::Reference(filespec_id));
    } else {
        catalog.set("AF", Object::Array(vec![Object::Reference(filespec_id)]));
    }

    // /Names -> /EmbeddedFiles Verzeichnis
    let mut names_entry = Dictionary::new();
    names_entry.set(
        "Names",
        Object::Array(vec![
            Object::String(b"factur-x.xml".to_vec(), StringFormat::Literal),
            Object::Reference(filespec_id),
        ]),
    );

    if let Ok(names_dict) = catalog.get_mut(b"Names").and_then(|n| n.as_dict_mut()) {
        names_dict.set("EmbeddedFiles", Object::Dictionary(names_entry));
    } else {
        let mut names = Dictionary::new();
        names.set("EmbeddedFiles", Object::Dictionary(names_entry));
        catalog.set("Names", Object::Dictionary(names));
    }

    Ok(())
}