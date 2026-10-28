export interface InvoiceTotals {
    line_total: number;
    tax_basis_total: number;
    tax_total: number;
    grand_total: number;
    due_payable: number;
}

export interface InvoiceItem {
    id: string; // Eindeutige ID für Svelte 5 {#each ... (item.id)} Keys
    name: string;
    quantity: number;
    unit_price: number;
    total_amount: number;
}

export interface ZugferdInvoice {
    profile: string;
    invoice_number: string;
    issue_date: string;
    currency: string;
    seller_name: string;
    buyer_name: string;
    iban: string | null;
    payment_reference: string | null;
    totals: InvoiceTotals;
    items: InvoiceItem[];
}

export interface ValidationIssue {
    rule_id: string;
    field: string;
    message: string;
    severity: 'error' | 'warning';
}

export interface ValidationReport {
    is_valid: boolean;
    error_count: number;
    warning_count: number;
    issues: ValidationIssue[];
}