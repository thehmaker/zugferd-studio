use crate::zugferd_parser::ZugferdInvoice;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn export_invoice_json<P: AsRef<Path>>(
    invoice: &ZugferdInvoice,
    target_path: P,
) -> Result<(), String> {
    let json_string = serde_json::to_string_pretty(invoice)
        .map_err(|e| format!("JSON-Serialisierungsfehler: {e}"))?;

    let mut file = File::create(target_path)
        .map_err(|e| format!("Datei konnte nicht erstellt werden: {e}"))?;

    file.write_all(json_string.as_bytes())
        .map_err(|e| format!("Schreibfehler: {e}"))?;

    Ok(())
}

pub fn export_invoice_csv<P: AsRef<Path>>(
    invoice: &ZugferdInvoice,
    target_path: P,
) -> Result<(), String> {
    let mut file = File::create(target_path)
        .map_err(|e| format!("Datei konnte nicht erstellt werden: {e}"))?;

    // UTF-8 BOM für fehlerfreie Zeichenerkennung (ä, ö, ü) in Microsoft Excel
    file.write_all(&[0xEF, 0xBB, 0xBF])
        .map_err(|e| format!("BOM-Schreibfehler: {e}"))?;

    let mut csv = String::new();

    // 1. Kopfdaten-Block
    csv.push_str("RECHNUNGSDATEN\n");
    csv.push_str(&format!("Rechnungsnummer;\"{}\"\n", escape_csv(&invoice.invoice_number)));
    csv.push_str(&format!("Rechnungsdatum;\"{}\"\n", escape_csv(&invoice.issue_date)));
    csv.push_str(&format!("Währung;\"{}\"\n", escape_csv(&invoice.currency)));
    csv.push_str(&format!("Verkäufer;\"{}\"\n", escape_csv(&invoice.seller_name)));
    csv.push_str(&format!("Käufer;\"{}\"\n", escape_csv(&invoice.buyer_name)));
    if let Some(iban) = &invoice.iban {
        csv.push_str(&format!("IBAN;\"{}\"\n", escape_csv(iban)));
    }
    csv.push('\n');

    // 2. Positionstabelle
    csv.push_str("Pos;Bezeichnung;Menge;Einzelpreis;Gesamtpreis\n");
    for (idx, item) in invoice.items.iter().enumerate() {
        csv.push_str(&format!(
            "{};\"{}\";{:.4};{:.2};{:.2}\n",
            idx + 1,
            escape_csv(&item.name),
            item.quantity,
            item.unit_price,
            item.total_amount
        ));
    }
    csv.push('\n');

    // 3. Summenblock
    csv.push_str(&format!("Nettobetrag;;;;{:.2}\n", invoice.totals.line_total));
    csv.push_str(&format!("Umsatzsteuer;;;;{:.2}\n", invoice.totals.tax_total));
    csv.push_str(&format!("Gesamtbetrag (Zahlbetrag);;;;{:.2}\n", invoice.totals.due_payable));

    file.write_all(csv.as_bytes())
        .map_err(|e| format!("CSV-Schreibfehler: {e}"))?;

    Ok(())
}

fn escape_csv(val: &str) -> String {
    val.replace('"', "\"\"")
}