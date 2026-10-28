<script lang="ts">
    import type { InvoiceItem, ValidationReport, ZugferdInvoice } from '$lib/types/invoice';
    import { invoke } from '@tauri-apps/api/core';
    import { save } from '@tauri-apps/plugin-dialog';
    import InvoicePrintModal from './InvoicePrintModal.svelte';

    interface Props {
        invoice: ZugferdInvoice;
        onSave?: (updatedInvoice: ZugferdInvoice) => void;
    }

    let { invoice, onSave }: Props = $props();
    let showPrintModal = $state(false);
    let isExporting = $state(false);

    let isEditing = $state(false);
    let taxRatePercent = $state(19.0);
    let validationReport = $state<ValidationReport | null>(null);

    // Lokaler Arbeitszustand für den Editiermodus
    let formInvoiceNumber = $state('');
    let formIssueDate = $state('');
    let formCurrency = $state('EUR');
    let formSellerName = $state('');
    let formBuyerName = $state('');
    let formIban = $state('');
    let formItems = $state<InvoiceItem[]>([]);

    // Svelte 5 $derived: Live-Berechnungen der Summen aus den Positionen
    let computedLineTotal = $derived.by(() => {
        return formItems.reduce((acc, item) => acc + (item.quantity * item.unit_price), 0);
    });

    let computedTaxTotal = $derived.by(() => {
        return computedLineTotal * (taxRatePercent / 100);
    });

    let computedGrandTotal = $derived.by(() => {
        return computedLineTotal + computedTaxTotal;
    });

        // Validierung beim Dokumentwechsel oder Änderungen anstoßen
    $effect(() => {
        // Wenn nicht im Editiermodus, die Prop-Rechnung validieren
        if (!isEditing && invoice) {
        runValidation(invoice);
        }
    });

    // Im Editiermodus: Bei Änderungen an den Inputs live validieren
    $effect(() => {
        if (isEditing) {
        // Reaktivität auf Formularfelder registrieren
        const draftInvoice: ZugferdInvoice = {
            ...invoice,
            invoice_number: formInvoiceNumber,
            issue_date: formIssueDate,
            currency: formCurrency,
            seller_name: formSellerName,
            buyer_name: formBuyerName,
            iban: formIban.trim() || null,
            totals: {
            line_total: computedLineTotal,
            tax_basis_total: computedLineTotal,
            tax_total: computedTaxTotal,
            grand_total: computedGrandTotal,
            due_payable: computedGrandTotal,
            },
            items: formItems.map((item) => ({
            ...item,
            total_amount: item.quantity * item.unit_price,
            })),
        };

        runValidation(draftInvoice);
        }
    });

    // Beim Start des Editiermodus Formulardaten aus der Prop initialisieren
    function enterEditMode() {
        formInvoiceNumber = invoice.invoice_number;
        formIssueDate = invoice.issue_date;
        formCurrency = invoice.currency || 'EUR';
        formSellerName = invoice.seller_name;
        formBuyerName = invoice.buyer_name;
        formIban = invoice.iban ?? '';

        // Tiefenkopie der Items mit eindeutiger ID für das DOM-Keying
        formItems = invoice.items.map((item, idx) => ({
        ...item,
        id: `item-${Date.now()}-${idx}`,
        }));

        // Falls die Rechnung bereits Steuer enthält, Satz annähern
        if (invoice.totals.line_total > 0 && invoice.totals.tax_total > 0) {
        taxRatePercent = Math.round((invoice.totals.tax_total / invoice.totals.line_total) * 100);
        }

        isEditing = true;
    }

    function cancelEdit() {
        isEditing = false;
    }

    function applyChanges() {
        const updatedInvoice: ZugferdInvoice = {
        ...invoice,
        invoice_number: formInvoiceNumber,
        issue_date: formIssueDate,
        currency: formCurrency,
        seller_name: formSellerName,
        buyer_name: formBuyerName,
        iban: formIban.trim().length > 0 ? formIban.trim() : null,
        totals: {
            line_total: computedLineTotal,
            tax_basis_total: computedLineTotal,
            tax_total: computedTaxTotal,
            grand_total: computedGrandTotal,
            due_payable: computedGrandTotal,
        },
        items: formItems.map((item) => ({
            ...item,
            total_amount: item.quantity * item.unit_price,
        })),
        };

        onSave?.(updatedInvoice);
        isEditing = false;
    }

    function addItem() {
        formItems.push({
        id: `item-${Date.now()}-${Math.random()}`,
        name: 'Neue Position',
        quantity: 1,
        unit_price: 0.0,
        total_amount: 0.0,
        });
    }

    function removeItem(id: string) {
        formItems = formItems.filter((i) => i.id !== id);
    }

    function formatMoney(amount: number, currency: string = 'EUR') {
        return new Intl.NumberFormat('de-DE', {
        style: 'currency',
        currency: currency || 'EUR',
        }).format(amount);
    }

    async function runValidation(inv: ZugferdInvoice) {
        try {
        validationReport = await invoke<ValidationReport>('validate_zugferd', { invoice: inv });
        } catch (e) {
        console.error('Validierungsfehler:', e);
        }
    }

    async function handleExportFile(format: 'json' | 'csv') {
    if (!invoice) return;

    try {
        const ext = format === 'json' ? 'json' : 'csv';
        const targetPath = await save({
            title: `Rechnung als ${format.toUpperCase()} exportieren`,
            defaultPath: `${invoice.invoice_number || 'rechnung'}.${ext}`,
            filters: [
            {
                name: `${format.toUpperCase()} Datei`,
                extensions: [ext],
            },
            ],
        });

        if (!targetPath) return;

        isExporting = true;
        await invoke('export_invoice', {
            invoice,
            format,
            targetPath,
        });
        } catch (e) {
        console.error(`Export-Fehler (${format}):`, e);
        } finally {
        isExporting = false;
        }
    }
    </script>

    <div class="editor-container">
        <!-- Obere Aktionsleiste des Inspectors -->
        <div class="action-header">
            <span class="profile-badge">{invoice.profile}</span>
            <div class="header-buttons">
            {#if isEditing}
                <button class="btn-secondary" onclick={cancelEdit}>Abbrechen</button>
                <button
                class="btn-success"
                onclick={applyChanges}
                disabled={validationReport && !validationReport.is_valid}
                title={validationReport && !validationReport.is_valid ? 'Bitte zuerst Validierungsfehler beheben' : ''}
                >
                Speichern
                </button>
            {:else}
                <button class="btn-edit" onclick={enterEditMode}>Bearbeiten</button>
            {/if}
            </div>
        </div>

        <!-- Export-Aktionsleiste im Inspector -->
        <div class="card export-bar">
            <span class="field-label">Export & Weiterverarbeitung</span>
            <div class="export-buttons">
            <button class="btn-subtle-sm" onclick={() => handleExportFile('json')} disabled={isExporting}>
                Export JSON
            </button>
            <button class="btn-subtle-sm" onclick={() => handleExportFile('csv')} disabled={isExporting}>
                Export CSV
            </button>
            <button class="btn-subtle-sm btn-print" onclick={() => (showPrintModal = true)}>
                🖨️ Druckansicht
            </button>
            </div>
        </div>

        <!-- Modal am Ende der Komponente -->
        <InvoicePrintModal
            {invoice}
            isOpen={showPrintModal}
            onClose={() => (showPrintModal = false)}
        />

        <!-- Validierungs-Banner -->
        {#if validationReport}
            {#if validationReport.is_valid && validationReport.warning_count === 0}
            <div class="validation-banner valid">
                <span>✓ EN 16931 konform</span>
            </div>
            {:else}
            <div class="validation-banner" class:has-errors={!validationReport.is_valid}>
                <div class="val-header">
                <strong>
                    {validationReport.is_valid ? 'Hinweise vorhanden' : 'Validierung fehlgeschlagen'}
                </strong>
                <span class="val-counts">
                    {#if validationReport.error_count > 0}
                    <span class="badge-err">{validationReport.error_count} Fehler</span>
                    {/if}
                    {#if validationReport.warning_count > 0}
                    <span class="badge-warn">{validationReport.warning_count} Warnungen</span>
                    {/if}
                </span>
                </div>
                <ul class="val-list">
                {#each validationReport.issues as issue}
                    <li class={issue.severity}>
                    <span class="rule-code">{issue.rule_id}</span>
                    <span>{issue.message}</span>
                    </li>
                {/each}
                </ul>
            </div>
            {/if}
        {/if}

        {#if !isEditing}
            <!-- ================= ANSICHTS-MODUS ================= -->
            <div class="card">
            <div class="title-row">
                <h4>{invoice.invoice_number ? `Rechnung ${invoice.invoice_number}` : 'Rechnung ohne Nr.'}</h4>
                <span class="muted-text">{invoice.issue_date}</span>
            </div>
            </div>

            <div class="card stack-gap">
            <div>
                <span class="field-label">Verkäufer</span>
                <strong class="entity-name">{invoice.seller_name}</strong>
                {#if invoice.iban}
                <span class="iban-text">IBAN: {invoice.iban}</span>
                {/if}
            </div>
            <div class="divider"></div>
            <div>
                <span class="field-label">Käufer</span>
                <strong class="entity-name">{invoice.buyer_name}</strong>
            </div>
            </div>

            <div class="card stack-gap">
            <span class="field-label">Summen</span>
            <div class="flex-between">
                <span>Nettobetrag:</span>
                <span>{formatMoney(invoice.totals.line_total, invoice.currency)}</span>
            </div>
            <div class="flex-between">
                <span>Umsatzsteuer:</span>
                <span>{formatMoney(invoice.totals.tax_total, invoice.currency)}</span>
            </div>
            <div class="flex-between total-highlight">
                <strong>Zahlbetrag:</strong>
                <strong>{formatMoney(invoice.totals.due_payable, invoice.currency)}</strong>
            </div>
            </div>

            {#if invoice.items.length > 0}
            <div class="card">
                <span class="field-label">Positionen ({invoice.items.length})</span>
                <div class="items-container">
                {#each invoice.items as item (item.name + item.total_amount)}
                    <div class="item-card">
                    <div class="item-main">
                        <span class="item-title">{item.name}</span>
                        <span class="item-calc">{item.quantity} × {formatMoney(item.unit_price, invoice.currency)}</span>
                    </div>
                    <span class="item-sum">{formatMoney(item.total_amount, invoice.currency)}</span>
                    </div>
                {/each}
                </div>
            </div>
            {/if}

        {:else}
            <!-- ================= EDITIER-MODUS ================= -->
            <div class="card stack-gap">
            <span class="field-label">Kopfdaten</span>
            <div class="form-row">
                <label>
                <span>Rechnungsnummer</span>
                <input type="text" bind:value={formInvoiceNumber} />
                </label>
                <label style="width: 110px;">
                <span>Datum</span>
                <input type="text" bind:value={formIssueDate} placeholder="TT.MM.JJJJ" />
                </label>
            </div>
            <div class="form-row">
                <label>
                <span>Währung</span>
                <input type="text" bind:value={formCurrency} maxlength="3" style="text-transform: uppercase;" />
                </label>
                <label>
                <span>USt. Satz (%)</span>
                <input type="number" step="0.5" bind:value={taxRatePercent} />
                </label>
            </div>
        </div>

        <div class="card stack-gap">
            <span class="field-label">Parteien</span>
            <label>
                <span>Verkäufer / Rechnungssteller</span>
                <input type="text" bind:value={formSellerName} />
            </label>
            <label>
                <span>IBAN des Verkäufers</span>
                <input type="text" bind:value={formIban} placeholder="DE..." />
            </label>
            <div class="divider"></div>
            <label>
                <span>Käufer / Rechnungsempfänger</span>
                <input type="text" bind:value={formBuyerName} />
            </label>
        </div>

        <div class="card stack-gap">
            <div class="flex-between">
                <span class="field-label">Positionen ({formItems.length})</span>
                <button class="btn-subtle-sm" onclick={addItem}>+ Position</button>
            </div>

            <div class="items-container">
                {#each formItems as item, idx (item.id)}
                <div class="item-edit-box">
                    <div class="flex-between">
                    <span class="pos-index">#{idx + 1}</span>
                    <button class="btn-delete" onclick={() => removeItem(item.id)} title="Löschen">✕</button>
                    </div>
                    <input type="text" bind:value={item.name} placeholder="Positionsbezeichnung" />
                    <div class="item-numbers-grid">
                    <label>
                        <span>Menge</span>
                        <input type="number" step="1" min="0" bind:value={item.quantity} />
                    </label>
                    <label>
                        <span>Einzelpreis</span>
                        <input type="number" step="0.01" min="0" bind:value={item.unit_price} />
                    </label>
                    <div class="pos-total">
                        <span class="label-tiny">Gesamt</span>
                        <span>{formatMoney(item.quantity * item.unit_price, formCurrency)}</span>
                    </div>
                    </div>
                </div>
                {/each}
            </div>
        </div>

        <!-- Live berechnete Summenbox -->
        <div class="card totals-preview">
            <span class="field-label">Berechnete Summen (Live)</span>
            <div class="flex-between">
                <span>Nettobetrag:</span>
                <span>{formatMoney(computedLineTotal, formCurrency)}</span>
            </div>
            <div class="flex-between">
                <span>MwSt ({taxRatePercent}%):</span>
                <span>{formatMoney(computedTaxTotal, formCurrency)}</span>
            </div>
            <div class="flex-between total-highlight">
                <strong>Neuer Zahlbetrag:</strong>
                <strong>{formatMoney(computedGrandTotal, formCurrency)}</strong>
            </div>
        </div>
        {/if}
    </div>

    <style>
    .editor-container {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .action-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding-bottom: 0.25rem;
    }

    .header-buttons {
        display: flex;
        gap: 0.5rem;
    }

    .card {
        background-color: #1c1c21;
        border: 1px solid #27272a;
        border-radius: 8px;
        padding: 0.85rem;
    }

    .totals-preview {
        border-color: #3b82f640;
        background-color: #1e2433;
    }

    .stack-gap {
        display: flex;
        flex-direction: column;
        gap: 0.6rem;
    }

    .profile-badge {
        display: inline-block;
        background-color: #065f46;
        color: #6ee7b7;
        font-size: 0.7rem;
        font-weight: 700;
        padding: 0.2rem 0.5rem;
        border-radius: 4px;
    }

    .title-row {
        display: flex;
        justify-content: space-between;
        align-items: baseline;
    }

    .title-row h4 {
        margin: 0;
        font-size: 0.95rem;
        color: #fafafa;
    }

    .muted-text {
        font-size: 0.8rem;
        color: #a1a1aa;
    }

    .field-label {
        display: block;
        font-size: 0.7rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: #71717a;
    }

    .entity-name {
        display: block;
        font-size: 0.9rem;
        color: #f4f4f5;
    }

    .iban-text {
        display: block;
        font-size: 0.75rem;
        color: #a1a1aa;
        font-family: monospace;
        margin-top: 0.2rem;
    }

    .divider {
        height: 1px;
        background-color: #27272a;
    }

    .flex-between {
        display: flex;
        justify-content: space-between;
        align-items: center;
        font-size: 0.85rem;
        color: #a1a1aa;
    }

    .total-highlight {
        margin-top: 0.3rem;
        padding-top: 0.3rem;
        border-top: 1px solid #27272a;
        color: #fafafa;
        font-size: 0.95rem;
    }

    .items-container {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
        margin-top: 0.4rem;
    }

    .item-card {
        display: flex;
        justify-content: space-between;
        align-items: center;
        background-color: #141417;
        padding: 0.5rem 0.6rem;
        border-radius: 4px;
    }

    .item-main {
        display: flex;
        flex-direction: column;
    }

    .item-title {
        font-size: 0.8rem;
        color: #fafafa;
    }

    .item-calc {
        font-size: 0.7rem;
        color: #71717a;
    }

    .item-sum {
        font-size: 0.8rem;
        font-weight: 600;
    }

    /* Formular & Input-Elemente */
    label {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    label span {
        font-size: 0.7rem;
        color: #a1a1aa;
    }

    .form-row {
        display: flex;
        gap: 0.5rem;
    }

    .form-row label {
        flex: 1;
    }

    input[type="text"],
    input[type="number"] {
        background-color: #121215;
        border: 1px solid #3f3f46;
        border-radius: 4px;
        color: #f4f4f5;
        padding: 0.4rem 0.5rem;
        font-size: 0.8rem;
        outline: none;
        transition: border-color 0.15s;
    }

    input:focus {
        border-color: #6366f1;
    }

    .item-edit-box {
        background-color: #141417;
        border: 1px solid #2e2e38;
        border-radius: 6px;
        padding: 0.6rem;
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
    }

    .pos-index {
        font-size: 0.7rem;
        font-weight: 700;
        color: #71717a;
    }

    .btn-delete {
        background: transparent;
        color: #ef4444;
        border: none;
        cursor: pointer;
        font-size: 0.8rem;
        padding: 0.1rem 0.3rem;
    }

    .item-numbers-grid {
        display: grid;
        grid-template-columns: 1fr 1fr 1.2fr;
        gap: 0.4rem;
        align-items: center;
    }

    .pos-total {
        display: flex;
        flex-direction: column;
        text-align: right;
        font-size: 0.8rem;
        font-weight: 600;
        color: #fafafa;
    }

    .label-tiny {
        font-size: 0.65rem;
        color: #71717a;
    }

    /* Buttons */
    button {
        font-size: 0.75rem;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
        padding: 0.35rem 0.7rem;
        border: none;
        transition: background-color 0.15s;
    }

    .btn-edit {
        background-color: #27272a;
        color: #e4e4e7;
        border: 1px solid #3f3f46;
    }

    .btn-edit:hover {
        background-color: #3f3f46;
    }

    .btn-secondary {
        background-color: #27272a;
        color: #a1a1aa;
    }

    .btn-success {
        background-color: #059669;
        color: white;
    }

    .btn-success:hover {
        background-color: #047857;
    }

    .btn-subtle-sm {
        background-color: #27272a;
        color: #818cf8;
        padding: 0.2rem 0.5rem;
        font-size: 0.7rem;
    }

    .validation-banner {
        background-color: #262216;
        border: 1px solid #785a18;
        border-radius: 6px;
        padding: 0.6rem 0.8rem;
        font-size: 0.75rem;
    }

    .validation-banner.valid {
        background-color: #064e3b20;
        border-color: #059669;
        color: #6ee7b7;
        font-weight: 600;
    }

    .validation-banner.has-errors {
        background-color: #3b1717;
        border-color: #991b1b;
    }

    .val-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 0.4rem;
    }

    .val-counts {
        display: flex;
        gap: 0.3rem;
    }

    .badge-err {
        background-color: #ef4444;
        color: white;
        padding: 0.1rem 0.35rem;
        border-radius: 3px;
        font-size: 0.65rem;
        font-weight: bold;
    }

    .badge-warn {
        background-color: #f59e0b;
        color: black;
        padding: 0.1rem 0.35rem;
        border-radius: 3px;
        font-size: 0.65rem;
        font-weight: bold;
    }

    .val-list {
        margin: 0;
        padding-left: 1rem;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .val-list li {
        color: #d4d4d8;
    }

    .val-list li.error {
        color: #fca5a5;
    }

    .val-list li.warning {
        color: #fde68a;
    }

    .rule-code {
        font-family: monospace;
        font-weight: bold;
        margin-right: 0.3rem;
        opacity: 0.8;
    }

    .export-bar {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
    }

    .export-buttons {
        display: grid;
        grid-template-columns: 1fr 1.2fr 1.2fr;
        gap: 0.4rem;
    }

    .btn-print {
        color: #34d399;
    }
</style>