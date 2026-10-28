use crate::zugferd_parser::ZugferdInvoice;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub rule_id: String,
    pub field: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub issues: Vec<ValidationIssue>,
}

pub fn validate_invoice(invoice: &ZugferdInvoice) -> ValidationReport {
    let mut issues = Vec::new();

    // ==========================================
    // 1. KOPFDATEN & FORMALE REGELN (BT-1 bis BT-5)
    // ==========================================

    // BR-02: Rechnungsnummer (BT-1) zwingend
    if invoice.invoice_number.trim().is_empty() {
        issues.push(ValidationIssue {
            rule_id: "BR-02".to_string(),
            field: "invoice_number".to_string(),
            message: "Eine Rechnungsnummer (BT-1) muss angegeben werden.".to_string(),
            severity: Severity::Error,
        });
    }

    // BR-03: Rechnungsdatum (BT-2) zwingend & formatiert
    if invoice.issue_date.trim().is_empty() {
        issues.push(ValidationIssue {
            rule_id: "BR-03".to_string(),
            field: "issue_date".to_string(),
            message: "Ein Rechnungsdatum (BT-2) ist verpflichtend.".to_string(),
            severity: Severity::Error,
        });
    } else if !is_valid_date_format(&invoice.issue_date) {
        issues.push(ValidationIssue {
            rule_id: "BR-03-FMT".to_string(),
            field: "issue_date".to_string(),
            message: "Rechnungsdatum muss im Format DD.MM.YYYY oder YYYYMMDD vorliegen.".to_string(),
            severity: Severity::Error,
        });
    }

    // BR-05: Währungscode (BT-5) 3-stellig nach ISO 4217
    let curr = invoice.currency.trim();
    if curr.len() != 3 || !curr.chars().all(|c| c.is_ascii_alphabetic()) {
        issues.push(ValidationIssue {
            rule_id: "BR-05".to_string(),
            field: "currency".to_string(),
            message: "Die Rechnungswährung (BT-5) muss ein 3-stelliger ISO-4217-Code sein (z. B. EUR).".to_string(),
            severity: Severity::Error,
        });
    }

    // ==========================================
    // 2. PARTEIEN: VERKÄUFER & KÄUFER
    // ==========================================

    // BR-06: Verkäufername (BT-27)
    if invoice.seller_name.trim().is_empty() || invoice.seller_name.trim() == "Unbekannt" {
        issues.push(ValidationIssue {
            rule_id: "BR-06".to_string(),
            field: "seller_name".to_string(),
            message: "Der vollständige Name des Verkäufers (BT-27) muss angegeben sein.".to_string(),
            severity: Severity::Error,
        });
    }

    // BR-07: Käufername (BT-44)
    if invoice.buyer_name.trim().is_empty() || invoice.buyer_name.trim() == "Unbekannt" {
        issues.push(ValidationIssue {
            rule_id: "BR-07".to_string(),
            field: "buyer_name".to_string(),
            message: "Der vollständige Name des Käufers (BT-44) muss angegeben sein.".to_string(),
            severity: Severity::Error,
        });
    }

    // Warnung: IBAN fehlt
    if invoice.iban.as_deref().unwrap_or("").trim().is_empty() {
        issues.push(ValidationIssue {
            rule_id: "BR-DE-19".to_string(),
            field: "iban".to_string(),
            message: "Es ist keine IBAN hinterlegt. Eine Überweisung ist so nicht automatisiert möglich.".to_string(),
            severity: Severity::Warning,
        });
    }

    // ==========================================
    // 3. RECHNUNGSPOSITIONEN
    // ==========================================

    // BR-16: Mindestens eine Position erforderlich
    if invoice.items.is_empty() {
        issues.push(ValidationIssue {
            rule_id: "BR-16".to_string(),
            field: "items".to_string(),
            message: "Eine Rechnung muss mindestens eine Position enthalten.".to_string(),
            severity: Severity::Error,
        });
    }

    let mut sum_lines = 0.0;

    for (idx, item) in invoice.items.iter().enumerate() {
        let pos_no = idx + 1;

        if item.name.trim().is_empty() {
            issues.push(ValidationIssue {
                rule_id: "BR-25".to_string(),
                field: format!("items[{idx}].name"),
                message: format!("Position #{pos_no} hat keine Artikelbezeichnung."),
                severity: Severity::Error,
            });
        }

        if item.quantity <= 0.0 {
            issues.push(ValidationIssue {
                rule_id: "BR-22".to_string(),
                field: format!("items[{idx}].quantity"),
                message: format!("Position #{pos_no}: Rechnungsmenge muss größer als 0 sein."),
                severity: Severity::Error,
            });
        }

        let calc_item_total = item.quantity * item.unit_price;
        if (calc_item_total - item.total_amount).abs() > 0.02 {
            issues.push(ValidationIssue {
                rule_id: "BR-CO-04".to_string(),
                field: format!("items[{idx}].total_amount"),
                message: format!(
                    "Position #{pos_no}: Zeilengesamtbetrag ({:.2}) stimmt nicht mit Menge × Einzelpreis ({:.2}) überein.",
                    item.total_amount, calc_item_total
                ),
                severity: Severity::Error,
            });
        }

        sum_lines += item.total_amount;
    }

    // ==========================================
    // 4. MATHEMATISCHE KONSISTENZ (SUMMATION)
    // ==========================================

    // BR-CO-10: Summe der Positionen = Nettogesamtbetrag
    if (sum_lines - invoice.totals.line_total).abs() > 0.02 {
        issues.push(ValidationIssue {
            rule_id: "BR-CO-10".to_string(),
            field: "totals.line_total".to_string(),
            message: format!(
                "Nettobetrag ({:.2}) weicht von der Summe der Einzelpositionen ({:.2}) ab.",
                invoice.totals.line_total, sum_lines
            ),
            severity: Severity::Error,
        });
    }

    // BR-CO-15: Netto + Steuer = Brutto/Zahlbetrag
    let calc_grand_total = invoice.totals.line_total + invoice.totals.tax_total;
    let expected_due = if invoice.totals.due_payable != 0.0 {
        invoice.totals.due_payable
    } else {
        invoice.totals.grand_total
    };

    if (calc_grand_total - expected_due).abs() > 0.02 {
        issues.push(ValidationIssue {
            rule_id: "BR-CO-15".to_string(),
            field: "totals.due_payable".to_string(),
            message: format!(
                "Zahlbetrag ({:.2}) entspricht nicht Netto + MwSt ({:.2}).",
                expected_due, calc_grand_total
            ),
            severity: Severity::Error,
        });
    }

    // Zusammenfassung ermitteln
    let error_count = issues.iter().filter(|i| i.severity == Severity::Error).count();
    let warning_count = issues.iter().filter(|i| i.severity == Severity::Warning).count();

    ValidationReport {
        is_valid: error_count == 0,
        error_count,
        warning_count,
        issues,
    }
}

fn is_valid_date_format(s: &str) -> bool {
    let trimmed = s.trim();
    // YYYYMMDD
    if trimmed.len() == 8 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    // DD.MM.YYYY
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() == 3 && parts[2].len() == 4 {
        return true;
    }
    // YYYY-MM-DD
    let parts_dash: Vec<&str> = trimmed.split('-').collect();
    if parts_dash.len() == 3 && parts_dash[0].len() == 4 {
        return true;
    }
    false
}