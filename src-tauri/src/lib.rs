use serde::{Deserialize, Serialize};

// Dieses Struct wird automatisch nach TypeScript gemappt
#[derive(Debug, Serialize, Deserialize)]
pub struct InvoiceSummary {
    pub invoice_number: String,
    pub total_amount: f64,
}

mod zugferd;
mod zugferd_export;
mod zugferd_generator;
mod zugferd_parser;
mod zugferd_stripper;
mod zugferd_validator;
mod zugferd_writer;

use std::fs;
use zugferd::{inspect_pdf, ZugferdDetection};
use zugferd_generator::generate_cii_xml;
use zugferd_export::{export_invoice_csv, export_invoice_json};
use zugferd_parser::ZugferdInvoice;
use zugferd_stripper::strip_zugferd_from_pdf;
use zugferd_validator::{validate_invoice, ValidationReport};
use zugferd_writer::embed_or_replace_xml;

use crate::zugferd_parser::parse_cii_xml;

#[tauri::command]
fn check_zugferd(path: String) -> Result<ZugferdDetection, String> {
    inspect_pdf(path)
}

#[tauri::command]
fn export_invoice(
    invoice: ZugferdInvoice,
    format: String,
    target_path: String,
) -> Result<(), String> {
    match format.as_str() {
        "json" => export_invoice_json(&invoice, target_path),
        "csv" => export_invoice_csv(&invoice, target_path),
        _ => Err("Nicht unterstütztes Exportformat".to_string()),
    }
}

#[tauri::command]
fn read_pdf_bytes(path: String) -> Result<Vec<u8>, String> {
    fs::read(&path).map_err(|e| format!("Datei konnte nicht gelesen werden: {e}"))
}

#[tauri::command]
fn generate_zugferd_xml(invoice: ZugferdInvoice) -> Result<String, String> {
    Ok(generate_cii_xml(&invoice))
}

#[tauri::command]
fn strip_zugferd(source_path: String, target_path: String) -> Result<bool, String> {
    strip_zugferd_from_pdf(source_path, target_path)
}

#[tauri::command]
fn validate_zugferd(invoice: ZugferdInvoice) -> Result<ValidationReport, String> {
    Ok(validate_invoice(&invoice))
}

#[tauri::command]
fn parse_external_xml(xml_content: String) -> Result<ZugferdInvoice, String> {
    parse_cii_xml(&xml_content)
}

#[tauri::command]
fn save_zugferd_pdf(
    source_path: String,
    target_path: String,
    xml_content: String,
    profile: Option<String>,
) -> Result<(), String> {
    let resolved_profile = profile.unwrap_or_else(|| "EN 16931 (COMFORT)".to_string());
    embed_or_replace_xml(source_path, target_path, &xml_content, &resolved_profile)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // Commands müssen hier im Handler registriert werden
        .invoke_handler(tauri::generate_handler![check_zugferd, export_invoice, read_pdf_bytes, generate_zugferd_xml, validate_zugferd, strip_zugferd, parse_external_xml, save_zugferd_pdf])
        .run(tauri::generate_context!())
        .expect("Fehler beim Starten der Tauri-App");
}
