<script lang="ts">
    import type { ZugferdInvoice } from '$lib/types/invoice';

    interface Props {
        invoice: ZugferdInvoice;
        isOpen: boolean;
        onClose: () => void;
    }

    let { invoice, isOpen, onClose }: Props = $props();

    function handlePrint() {
        window.print();
    }

    function formatMoney(amount: number, curr: string = 'EUR') {
        return new Intl.NumberFormat('de-DE', {
        style: 'currency',
        currency: curr || 'EUR',
        }).format(amount);
    }
    </script>

    {#if isOpen}
    <div class="print-overlay" onclick={onClose} role="presentation">
        <div class="modal-window" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
        <!-- Obere Leiste (Wird beim Drucken ausgeblendet) -->
        <div class="no-print modal-bar">
            <span>Druckvorschau</span>
            <div class="bar-actions">
            <button class="btn-primary" onclick={handlePrint}>🖨️ Drucken / Als PDF speichern</button>
            <button class="btn-subtle" onclick={onClose}>Schließen</button>
            </div>
        </div>

        <!-- Das druckbare DIN A4 Rechnungsblatt -->
        <div class="printable-sheet">
            <header class="sheet-header">
            <div class="sender-info">
                <h2>{invoice.seller_name}</h2>
                {#if invoice.iban}
                <p class="small-text">IBAN: {invoice.iban}</p>
                {/if}
            </div>
            <div class="invoice-meta">
                <h1>RECHNUNG</h1>
                <p><strong>Nummer:</strong> {invoice.invoice_number}</p>
                <p><strong>Datum:</strong> {invoice.issue_date}</p>
                <p class="profile-tag">Standard: {invoice.profile}</p>
            </div>
            </header>

            <section class="recipient-box">
            <span class="label">Rechnungsempfänger:</span>
            <strong>{invoice.buyer_name}</strong>
            </section>

            <table class="items-table">
            <thead>
                <tr>
                <th style="width: 40px;">Pos</th>
                <th>Beschreibung</th>
                <th style="width: 80px; text-align: right;">Menge</th>
                <th style="width: 100px; text-align: right;">Einzelpreis</th>
                <th style="width: 110px; text-align: right;">Gesamt</th>
                </tr>
            </thead>
            <tbody>
                {#each invoice.items as item, idx}
                <tr>
                    <td>{idx + 1}</td>
                    <td>{item.name}</td>
                    <td style="text-align: right;">{item.quantity}</td>
                    <td style="text-align: right;">{formatMoney(item.unit_price, invoice.currency)}</td>
                    <td style="text-align: right; font-weight: 500;">
                    {formatMoney(item.total_amount, invoice.currency)}
                    </td>
                </tr>
                {/each}
            </tbody>
            </table>

            <section class="totals-section">
            <div class="totals-table">
                <div class="t-row">
                <span>Nettobetrag:</span>
                <span>{formatMoney(invoice.totals.line_total, invoice.currency)}</span>
                </div>
                <div class="t-row">
                <span>MwSt. gesamt:</span>
                <span>{formatMoney(invoice.totals.tax_total, invoice.currency)}</span>
                </div>
                <div class="t-row due-highlight">
                <strong>Zahlbetrag:</strong>
                <strong>{formatMoney(invoice.totals.due_payable, invoice.currency)}</strong>
                </div>
            </div>
            </section>

            <footer class="sheet-footer">
            <p>Diese Rechnung wurde elektronisch generiert und entspricht dem ZUGFeRD / Factur-X Standard.</p>
            </footer>
        </div>
        </div>
    </div>
    {/if}

    <style>
    .print-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.8);
        display: flex;
        justify-content: center;
        align-items: flex-start;
        padding: 2rem;
        overflow-y: auto;
        z-index: 1000;
    }

    .modal-window {
        display: flex;
        flex-direction: column;
        max-width: 860px;
        width: 100%;
    }

    .modal-bar {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.75rem 1rem;
        background: #18181b;
        border: 1px solid #27272a;
        border-radius: 8px 8px 0 0;
    }

    .bar-actions {
        display: flex;
        gap: 0.5rem;
    }

    /* A4-Blatt Style im Screen-Modus */
    .printable-sheet {
        background: #ffffff;
        color: #111827;
        padding: 3rem;
        min-height: 297mm;
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    }

    .sheet-header {
        display: flex;
        justify-content: space-between;
        border-bottom: 2px solid #e5e7eb;
        padding-bottom: 1.5rem;
    }

    .sheet-header h1 {
        margin: 0;
        font-size: 1.5rem;
        color: #1e3a8a;
    }

    .sheet-header h2 {
        margin: 0;
        font-size: 1.25rem;
    }

    .small-text {
        font-size: 0.8rem;
        color: #4b5563;
        margin: 0.25rem 0 0;
        font-family: monospace;
    }

    .invoice-meta {
        text-align: right;
    }

    .invoice-meta p {
        margin: 0.2rem 0;
        font-size: 0.85rem;
    }

    .profile-tag {
        font-size: 0.75rem;
        color: #059669;
        font-weight: 600;
    }

    .recipient-box {
        margin: 2.5rem 0;
        font-size: 0.95rem;
    }

    .recipient-box .label {
        display: block;
        font-size: 0.75rem;
        color: #6b7280;
        text-transform: uppercase;
    }

    .items-table {
        width: 100%;
        border-collapse: collapse;
        margin-top: 1rem;
        font-size: 0.85rem;
    }

    .items-table th {
        background-color: #f3f4f6;
        border-bottom: 2px solid #d1d5db;
        padding: 0.6rem 0.5rem;
        text-align: left;
    }

    .items-table td {
        border-bottom: 1px solid #e5e7eb;
        padding: 0.6rem 0.5rem;
    }

    .totals-section {
        display: flex;
        justify-content: flex-end;
        margin-top: 2rem;
    }

    .totals-table {
        width: 280px;
        display: flex;
        flex-direction: column;
        gap: 0.35rem;
        font-size: 0.85rem;
    }

    .t-row {
        display: flex;
        justify-content: space-between;
    }

    .due-highlight {
        border-top: 2px solid #111827;
        margin-top: 0.35rem;
        padding-top: 0.35rem;
        font-size: 1rem;
    }

    .sheet-footer {
        margin-top: 4rem;
        border-top: 1px solid #e5e7eb;
        padding-top: 1rem;
        font-size: 0.75rem;
        color: #6b7280;
        text-align: center;
    }

    .btn-primary {
        background-color: #4f46e5;
        color: white;
        border: none;
        border-radius: 6px;
        padding: 0.4rem 0.8rem;
        font-size: 0.8rem;
        cursor: pointer;
    }

    .btn-subtle {
        background: #27272a;
        color: #e4e4e7;
        border: none;
        border-radius: 6px;
        padding: 0.4rem 0.8rem;
        font-size: 0.8rem;
        cursor: pointer;
    }

    /* ================= DRUCK-STYLES (@media print) ================= */
    @media print {
        /* Versteckt alle UI-Fenster und Menüleisten der App */
        :global(body *) {
        visibility: hidden;
        }

        .printable-sheet,
        .printable-sheet * {
        visibility: visible;
        }

        .printable-sheet {
        position: absolute;
        left: 0;
        top: 0;
        width: 100%;
        min-height: auto;
        padding: 0;
        box-shadow: none;
        }

        .no-print {
        display: none !important;
        }
    }
</style>