use quick_xml::Reader;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceTotals {
    pub line_total: f64,
    pub tax_basis_total: f64,
    pub tax_total: f64,
    pub grand_total: f64,
    pub due_payable: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub total_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZugferdInvoice {
    pub profile: String,
    pub invoice_number: String,
    pub issue_date: String,
    pub currency: String,
    pub seller_name: String,
    pub buyer_name: String,
    pub iban: Option<String>,
    pub payment_reference: Option<String>,
    pub totals: InvoiceTotals,
    pub items: Vec<InvoiceItem>,
}

pub fn parse_cii_xml(xml: &str) -> Result<ZugferdInvoice, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut profile = String::from("UNBEKANNT");
    let mut invoice_number = String::new();
    let mut issue_date = String::new();
    let mut currency = String::from("EUR");
    let mut seller_name = String::new();
    let mut buyer_name = String::new();
    let mut iban: Option<String> = None;
    let mut payment_reference: Option<String> = None;

    let mut line_total = 0.0;
    let mut tax_basis_total = 0.0;
    let mut tax_total = 0.0;
    let mut grand_total = 0.0;
    let mut due_payable = 0.0;

    let mut items: Vec<InvoiceItem> = Vec::new();

    // Zustands-Tracking während des Streamens
    let mut tag_stack: Vec<String> = Vec::new();
    let mut in_seller = false;
    let mut in_buyer = false;
    let mut in_item = false;

    // Temporäre Felder für eine Position
    let mut cur_item_name = String::new();
    let mut cur_item_qty = 1.0;
    let mut cur_item_price = 0.0;
    let mut cur_item_total = 0.0;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                // .into_inner() liefert das saubere &[u8] aus dem QName
                let local_name = extract_local_name(e.name().into_inner().as_bytes());

                if local_name == "SellerTradeParty" {
                    in_seller = true;
                } else if local_name == "BuyerTradeParty" {
                    in_buyer = true;
                } else if local_name == "IncludedSupplyChainTradeLineItem" {
                    in_item = true;
                    cur_item_name.clear();
                    cur_item_qty = 1.0;
                    cur_item_price = 0.0;
                    cur_item_total = 0.0;
                }

                tag_stack.push(local_name.to_string());
            }
            Ok(Event::End(e)) => {
                // Auch hier .into_inner() nutzen
                let local_name = extract_local_name(e.name().into_inner().as_bytes());

                if local_name == "SellerTradeParty" {
                    in_seller = false;
                } else if local_name == "BuyerTradeParty" {
                    in_buyer = false;
                } else if local_name == "IncludedSupplyChainTradeLineItem" {
                    in_item = false;
                    items.push(InvoiceItem {
                        id: None,
                        name: if cur_item_name.is_empty() {
                            "Position".to_string()
                        } else {
                            cur_item_name.clone()
                        },
                        quantity: cur_item_qty,
                        unit_price: cur_item_price,
                        total_amount: cur_item_total,
                    });
                }

                tag_stack.pop();
            }
            Ok(Event::Text(e)) => {
                // 1. Raw Bytes in &str umwandeln
                let raw_str = match std::str::from_utf8(e.as_ref().as_bytes()) {
                    Ok(s) => s.trim(),
                    Err(_) => continue,
                };

                if raw_str.is_empty() {
                    continue;
                }

                // 2. XML-Entities auflösen (z.B. &amp; -> &)
                let text = quick_xml::escape::unescape(raw_str)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw_str.to_string());

                let current_tag = tag_stack.last().map(|s| s.as_str()).unwrap_or("");
                let parent_tag = if tag_stack.len() >= 2 {
                    tag_stack[tag_stack.len() - 2].as_str()
                } else {
                    ""
                };

                match current_tag {
                    "ID" if parent_tag == "GuidelineSpecifiedDocumentContextParameter" => {
                        profile = resolve_profile_name(&text);
                    }
                    "ID" if parent_tag == "ExchangedDocument" => {
                        invoice_number = text;
                    }
                    "DateTimeString" if issue_date.is_empty() => {
                        issue_date = format_date(&text);
                    }
                    "InvoiceCurrencyCode" => {
                        currency = text;
                    }
                    "Name" if in_seller && seller_name.is_empty() => {
                        seller_name = text;
                    }
                    "Name" if in_buyer && buyer_name.is_empty() => {
                        buyer_name = text;
                    }
                    "IBANID" => {
                        iban = Some(text);
                    }
                    "PaymentReference" => {
                        payment_reference = Some(text);
                    }
                    // Summen
                    "LineTotalAmount" if !in_item => {
                        line_total = text.parse().unwrap_or(0.0);
                    }
                    "TaxBasisTotalAmount" => {
                        tax_basis_total = text.parse().unwrap_or(0.0);
                    }
                    "TaxTotalAmount" => {
                        tax_total = text.parse().unwrap_or(0.0);
                    }
                    "GrandTotalAmount" => {
                        grand_total = text.parse().unwrap_or(0.0);
                    }
                    "DuePayableAmount" => {
                        due_payable = text.parse().unwrap_or(0.0);
                    }
                    // Item-Ebene
                    "Name" if in_item && cur_item_name.is_empty() => {
                        cur_item_name = text;
                    }
                    "BilledQuantity" if in_item => {
                        cur_item_qty = text.parse().unwrap_or(1.0);
                    }
                    "ChargeAmount" if in_item && cur_item_price == 0.0 => {
                        cur_item_price = text.parse().unwrap_or(0.0);
                    }
                    "LineTotalAmount" if in_item => {
                        cur_item_total = text.parse().unwrap_or(0.0);
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!(
                    "XML-Syntaxfehler bei Byte {}: {e}",
                    reader.buffer_position()
                ));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(ZugferdInvoice {
        profile,
        invoice_number,
        issue_date,
        currency,
        seller_name: if seller_name.is_empty() {
            "Unbekannt".into()
        } else {
            seller_name
        },
        buyer_name: if buyer_name.is_empty() {
            "Unbekannt".into()
        } else {
            buyer_name
        },
        iban,
        payment_reference,
        totals: InvoiceTotals {
            line_total,
            tax_basis_total,
            tax_total,
            grand_total,
            due_payable: if due_payable == 0.0 {
                grand_total
            } else {
                due_payable
            },
        },
        items,
    })
}

/// Extrahiert den lokalen Namen ohne Namespace-Präfix (z. B. "ram:ID" -> "ID")
fn extract_local_name(bytes: &[u8]) -> &str {
    let s = std::str::from_utf8(bytes).unwrap_or("");
    s.split(':').last().unwrap_or(s)
}

fn resolve_profile_name(urn: &str) -> String {
    let lower = urn.to_lowercase();
    if lower.contains("minimum") {
        "MINIMUM".to_string()
    } else if lower.contains("basicwl") {
        "BASIC WL".to_string()
    } else if lower.contains("basic") {
        "BASIC".to_string()
    } else if lower.contains("en16931") {
        "EN 16931 (COMFORT)".to_string()
    } else if lower.contains("extended") {
        "EXTENDED".to_string()
    } else if lower.contains("xrechnung") {
        "XRECHNUNG".to_string()
    } else {
        "CUSTOM".to_string()
    }
}

fn format_date(raw: &str) -> String {
    if raw.len() == 8 {
        // YYYYMMDD -> DD.MM.YYYY
        format!("{}.{}.{}", &raw[6..8], &raw[4..6], &raw[0..4])
    } else {
        raw.to_string()
    }
}
