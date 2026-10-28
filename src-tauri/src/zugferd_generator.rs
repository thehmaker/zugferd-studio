use crate::zugferd_parser::ZugferdInvoice;
use quick_xml::escape::escape;

pub fn generate_cii_xml(invoice: &ZugferdInvoice) -> String {
    let profile_urn = resolve_urn(&invoice.profile);
    let date_102 = convert_to_102(&invoice.issue_date);
    let currency = if invoice.currency.trim().is_empty() {
        "EUR"
    } else {
        invoice.currency.trim()
    };

    let mut xml = String::with_capacity(4096);

    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(
        "<rsm:CrossIndustryInvoice \
        xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\" \
        xmlns:ccts=\"urn:un:unece:uncefact:documentation:standard:CoreComponentsTechnicalSpecification:2\" \
        xmlns:udt=\"urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100\" \
        xmlns:qdt=\"urn:un:unece:uncefact:data:standard:QualifiedDataType:100\" \
        xmlns:ram=\"urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100\">\n",
    );

    // 1. Kontext & Profil
    xml.push_str("  <rsm:ExchangedDocumentContext>\n");
    xml.push_str("    <ram:GuidelineSpecifiedDocumentContextParameter>\n");
    xml.push_str(&format!("      <ram:ID>{}</ram:ID>\n", profile_urn));
    xml.push_str("    </ram:GuidelineSpecifiedDocumentContextParameter>\n");
    xml.push_str("  </rsm:ExchangedDocumentContext>\n");

    // 2. Dokumentenkopf
    xml.push_str("  <rsm:ExchangedDocument>\n");
    xml.push_str(&format!(
        "    <ram:ID>{}</ram:ID>\n",
        escape(&invoice.invoice_number)
    ));
    xml.push_str("    <ram:TypeCode>380</ram:TypeCode>\n"); // 380 = Handelsrechnung
    xml.push_str("    <ram:IssueDateTime>\n");
    xml.push_str(&format!(
        "      <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n",
        date_102
    ));
    xml.push_str("    </ram:IssueDateTime>\n");
    xml.push_str("  </rsm:ExchangedDocument>\n");

    // 3. Transaktionskörper
    xml.push_str("  <rsm:SupplyChainTradeTransaction>\n");

    // Positionen
    for (idx, item) in invoice.items.iter().enumerate() {
        let line_id = idx + 1;
        xml.push_str("    <ram:IncludedSupplyChainTradeLineItem>\n");
        xml.push_str("      <ram:AssociatedDocumentLineDocument>\n");
        xml.push_str(&format!("        <ram:LineID>{}</ram:LineID>\n", line_id));
        xml.push_str("      </ram:AssociatedDocumentLineDocument>\n");

        xml.push_str("      <ram:SpecifiedTradeProduct>\n");
        xml.push_str(&format!(
            "        <ram:Name>{}</ram:Name>\n",
            escape(&item.name)
        ));
        xml.push_str("      </ram:SpecifiedTradeProduct>\n");

        xml.push_str("      <ram:SpecifiedLineTradeAgreement>\n");
        xml.push_str("        <ram:NetPriceProductTradePrice>\n");
        xml.push_str(&format!(
            "          <ram:ChargeAmount>{:.2}</ram:ChargeAmount>\n",
            item.unit_price
        ));
        xml.push_str("        </ram:NetPriceProductTradePrice>\n");
        xml.push_str("      </ram:SpecifiedLineTradeAgreement>\n");

        xml.push_str("      <ram:SpecifiedLineTradeDelivery>\n");
        xml.push_str(&format!(
            "        <ram:BilledQuantity unitCode=\"C62\">{:.4}</ram:BilledQuantity>\n",
            item.quantity
        ));
        xml.push_str("      </ram:SpecifiedLineTradeDelivery>\n");

        xml.push_str("      <ram:SpecifiedLineTradeSettlement>\n");
        xml.push_str("        <ram:ApplicableTradeTax>\n");
        xml.push_str("          <ram:TypeCode>VAT</ram:TypeCode>\n");
        xml.push_str("          <ram:CategoryCode>S</ram:CategoryCode>\n");
        let rate = if invoice.totals.line_total > 0.0 {
            (invoice.totals.tax_total / invoice.totals.line_total) * 100.0
        } else {
            19.0
        };
        xml.push_str(&format!(
            "          <ram:RateApplicablePercent>{:.2}</ram:RateApplicablePercent>\n",
            rate
        ));
        xml.push_str("        </ram:ApplicableTradeTax>\n");
        xml.push_str("        <ram:SpecifiedTradeSettlementLineMonetarySummation>\n");
        xml.push_str(&format!(
            "          <ram:LineTotalAmount>{:.2}</ram:LineTotalAmount>\n",
            item.total_amount
        ));
        xml.push_str("        </ram:SpecifiedTradeSettlementLineMonetarySummation>\n");
        xml.push_str("      </ram:SpecifiedLineTradeSettlement>\n");
        xml.push_str("    </ram:IncludedSupplyChainTradeLineItem>\n");
    }

    // Parteien (Agreement)
    xml.push_str("    <ram:ApplicableHeaderTradeAgreement>\n");
    xml.push_str("      <ram:SellerTradeParty>\n");
    xml.push_str(&format!(
        "        <ram:Name>{}</ram:Name>\n",
        escape(&invoice.seller_name)
    ));
    xml.push_str("        <ram:PostalTradeAddress>\n");
    xml.push_str("          <ram:CountryID>DE</ram:CountryID>\n");
    xml.push_str("        </ram:PostalTradeAddress>\n");
    xml.push_str("      </ram:SellerTradeParty>\n");

    xml.push_str("      <ram:BuyerTradeParty>\n");
    xml.push_str(&format!(
        "        <ram:Name>{}</ram:Name>\n",
        escape(&invoice.buyer_name)
    ));
    xml.push_str("        <ram:PostalTradeAddress>\n");
    xml.push_str("          <ram:CountryID>DE</ram:CountryID>\n");
    xml.push_str("        </ram:PostalTradeAddress>\n");
    xml.push_str("      </ram:BuyerTradeParty>\n");
    xml.push_str("    </ram:ApplicableHeaderTradeAgreement>\n");

    // Delivery-Header
    xml.push_str("    <ram:ApplicableHeaderTradeDelivery/>\n");

    // Zahlungs- und Summenblock (Settlement)
    xml.push_str("    <ram:ApplicableHeaderTradeSettlement>\n");
    if let Some(payment_ref) = &invoice.payment_reference {
        if !payment_ref.trim().is_empty() {
            xml.push_str(&format!(
                "      <ram:PaymentReference>{}</ram:PaymentReference>\n",
                escape(payment_ref)
            ));
        }
    }
    xml.push_str(&format!(
        "      <ram:InvoiceCurrencyCode>{}</ram:InvoiceCurrencyCode>\n",
        currency
    ));

    // IBAN / Zahlungsziel
    if let Some(iban) = &invoice.iban {
        if !iban.trim().is_empty() {
            xml.push_str("      <ram:SpecifiedTradeSettlementPaymentMeans>\n");
            xml.push_str("        <ram:TypeCode>58</ram:TypeCode>\n"); // SEPA-Überweisung
            xml.push_str("        <ram:PayeePartyCreditorFinancialAccount>\n");
            xml.push_str(&format!(
                "          <ram:IBANID>{}</ram:IBANID>\n",
                escape(iban.trim())
            ));
            xml.push_str("        </ram:PayeePartyCreditorFinancialAccount>\n");
            xml.push_str("      </ram:SpecifiedTradeSettlementPaymentMeans>\n");
        }
    }

    // Steuerblock
    xml.push_str("      <ram:ApplicableTradeTax>\n");
    xml.push_str(&format!(
        "        <ram:CalculatedAmount>{:.2}</ram:CalculatedAmount>\n",
        invoice.totals.tax_total
    ));
    xml.push_str("        <ram:TypeCode>VAT</ram:TypeCode>\n");
    xml.push_str(&format!(
        "        <ram:BasisAmount>{:.2}</ram:BasisAmount>\n",
        invoice.totals.tax_basis_total
    ));
    xml.push_str("        <ram:CategoryCode>S</ram:CategoryCode>\n");
    let tax_rate = if invoice.totals.tax_basis_total > 0.0 {
        (invoice.totals.tax_total / invoice.totals.tax_basis_total) * 100.0
    } else {
        19.0
    };
    xml.push_str(&format!(
        "        <ram:RateApplicablePercent>{:.2}</ram:RateApplicablePercent>\n",
        tax_rate
    ));
    xml.push_str("      </ram:ApplicableTradeTax>\n");

    // Summation
    xml.push_str("      <ram:SpecifiedTradeSettlementHeaderMonetarySummation>\n");
    xml.push_str(&format!(
        "        <ram:LineTotalAmount>{:.2}</ram:LineTotalAmount>\n",
        invoice.totals.line_total
    ));
    xml.push_str(&format!(
        "        <ram:TaxBasisTotalAmount>{:.2}</ram:TaxBasisTotalAmount>\n",
        invoice.totals.tax_basis_total
    ));
    xml.push_str(&format!(
        "        <ram:TaxTotalAmount currencyID=\"{}\">{:.2}</ram:TaxTotalAmount>\n",
        currency, invoice.totals.tax_total
    ));
    xml.push_str(&format!(
        "        <ram:GrandTotalAmount>{:.2}</ram:GrandTotalAmount>\n",
        invoice.totals.grand_total
    ));
    xml.push_str(&format!(
        "        <ram:DuePayableAmount>{:.2}</ram:DuePayableAmount>\n",
        invoice.totals.due_payable
    ));
    xml.push_str("      </ram:SpecifiedTradeSettlementHeaderMonetarySummation>\n");

    xml.push_str("    </ram:ApplicableHeaderTradeSettlement>\n");
    xml.push_str("  </rsm:SupplyChainTradeTransaction>\n");
    xml.push_str("</rsm:CrossIndustryInvoice>\n");

    xml
}

fn resolve_urn(profile: &str) -> &'static str {
    let lower = profile.to_lowercase();
    if lower.contains("minimum") {
        "urn:factur-x.eu:1p0:minimum"
    } else if lower.contains("basicwl") || lower.contains("basic wl") {
        "urn:factur-x.eu:1p0:basicwl"
    } else if lower.contains("basic") {
        "urn:factur-x.eu:1p0:basic"
    } else if lower.contains("xrechnung") {
        "urn:cen.eu:en16931:2017#compliant#urn:xoev-de:kosit:standard:xrechnung_3.0"
    } else if lower.contains("extended") {
        "urn:cen.eu:en16931:2017#conformant#urn:factur-x.eu:1p0:extended"
    } else {
        // Standard: EN 16931 (COMFORT)
        "urn:cen.eu:en16931:2017"
    }
}

/// Konvertiert DD.MM.YYYY oder YYYY-MM-DD in Format 102 (YYYYMMDD)
fn convert_to_102(date_str: &str) -> String {
    let cleaned = date_str.trim();

    // Fall 1: DD.MM.YYYY
    let parts_dot: Vec<&str> = cleaned.split('.').collect();
    if parts_dot.len() == 3 && parts_dot[2].len() == 4 {
        return format!(
            "{}{:0>2}{:0>2}",
            parts_dot[2], parts_dot[1], parts_dot[0]
        );
    }

    // Fall 2: YYYY-MM-DD
    let parts_dash: Vec<&str> = cleaned.split('-').collect();
    if parts_dash.len() == 3 && parts_dash[0].len() == 4 {
        return format!(
            "{}{:0>2}{:0>2}",
            parts_dash[0], parts_dash[1], parts_dash[2]
        );
    }

    // Fall 3: Bereits 8 Ziffern
    if cleaned.len() == 8 && cleaned.chars().all(|c| c.is_ascii_digit()) {
        return cleaned.to_string();
    }

    // Fallback: 20261027
    "20261027".to_string()
}