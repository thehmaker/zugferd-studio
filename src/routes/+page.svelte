<script lang="ts">
  import AboutModal from '$lib/components/AboutModal.svelte';
  import PdfViewer from '$lib/components/PdfViewer.svelte';
  import ZugferdEditor from '$lib/components/ZugferdEditor.svelte';
  import type { InvoiceItem, ZugferdInvoice } from '$lib/types/invoice';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';

  let showAboutModal = $state(false);
  let isSaving = $state(false);
  let saveSuccessMessage = $state<string | null>(null);
  let isStripping = $state(false);
  let isDraggingOver = $state(false);

  onMount(() => {
    let unlistenDragDrop: (() => void) | undefined;

    // Registrierung des nativen Tauri v2 OS-Drag-and-Drop Listeners
    async function setupDragDrop() {
      const appWindow = getCurrentWebviewWindow();

      unlistenDragDrop = await appWindow.onDragDropEvent(async (event) => {
        const payload = event.payload;

        if (payload.type === 'enter' || payload.type === 'over') {
          isDraggingOver = true;
        } else if (payload.type === 'leave') {
          isDraggingOver = false;
        } else if (payload.type === 'drop') {
          isDraggingOver = false;

          const droppedPaths = payload.paths;
          if (!droppedPaths || droppedPaths.length === 0) return;

          // Erste Datei auswerten
          const filePath = droppedPaths[0];
          await handleDroppedFile(filePath);
        }
      });
    }

    setupDragDrop();

    return () => {
      if (unlistenDragDrop) unlistenDragDrop();
    };
  });

  interface ZugferdDetection {
    has_zugferd: boolean;
    filename: string | null;
    raw_xml: string | null;
    xml_size_bytes: number;
    invoice: ZugferdInvoice | null;
  }

  let selectedPdfPath = $state<string | null>(null);
  let pdfBytes = $state<Uint8Array | null>(null);
  let isProcessing = $state(false);
  let detection = $state<ZugferdDetection | null>(null);
  let errorMessage = $state<string | null>(null);
  let isDirty = $state(false);

  let fileName = $derived.by(() => {
    if (!selectedPdfPath) return null;
    return selectedPdfPath.split(/[/\\]/).pop() ?? selectedPdfPath;
  });

  async function handlePickFile() {
    errorMessage = null;
    try {
      const result = await open({
        multiple: false,
        directory: false,
        title: 'PDF-Rechnung auswählen',
        filters: [{ name: 'PDF Dokumente', extensions: ['pdf'] }],
      });

      if (typeof result === 'string') {
        selectedPdfPath = result;
        await loadAndAnalyze(result);
      }
    } catch (err) {
      errorMessage = String(err);
    }
  }

  async function handleDroppedFile(filePath: string) {
    errorMessage = null;
    const lower = filePath.toLowerCase();

    // Fall 1: PDF-Datei abgelegt
    if (lower.endsWith('.pdf')) {
      selectedPdfPath = filePath;
      await loadAndAnalyze(filePath);
      return;
    }

    // Fall 2: XML-Datei abgelegt
    if (lower.endsWith('.xml')) {
      if (!selectedPdfPath) {
        errorMessage = 'Bitte öffne zuerst ein PDF-Dokument, bevor du eine XML-Datei anhängst.';
        return;
      }

      try {
        // XML einlesen & parsen
        const rawBytes = await invoke<number[]>('read_pdf_bytes', { path: filePath });
        const xmlString = new TextDecoder('utf-8').decode(new Uint8Array(rawBytes));
        const parsedInvoice = await invoke<ZugferdInvoice>('parse_external_xml', {
          xmlContent: xmlString,
        });

        if (detection) {
          detection.has_zugferd = true;
          detection.filename = 'factur-x.xml';
          detection.invoice = parsedInvoice;
          detection.raw_xml = xmlString;
          detection.xml_size_bytes = rawBytes.length;
          isDirty = true;
        }
      } catch (err) {
        errorMessage = `Fehler beim Importieren der abgelegten XML: ${err}`;
      }
      return;
    }

    errorMessage = 'Nicht unterstütztes Dateiformat. Bitte lege eine .pdf- oder .xml-Datei ab.';
  }

  function createEmptyInvoice(): ZugferdInvoice {
    const today = new Date();
    const day = String(today.getDate()).padStart(2, '0');
    const month = String(today.getMonth() + 1).padStart(2, '0');
    const year = today.getFullYear();

    return {
      profile: 'EN 16931 (COMFORT)',
      invoice_number: `RE-${year}-001`,
      issue_date: `${day}.${month}.${year}`,
      currency: 'EUR',
      seller_name: 'Meine Firma GmbH',
      buyer_name: 'Musterkunde AG',
      iban: 'DE',
      payment_reference: `RE-${year}-001`,
      totals: {
        line_total: 100.0,
        tax_basis_total: 100.0,
        tax_total: 19.0,
        grand_total: 119.0,
        due_payable: 119.0,
      },
      items: [
        {
          id: `item-${Date.now()}`,
          name: 'Dienstleistung / Beratung',
          quantity: 1,
          unit_price: 100.0,
          total_amount: 100.0,
        },
      ],
    };
  }

  // Option A: Leere Rechnung initialisieren
  async function handleCreateNewInvoice() {
    if (!detection) return;

    const newInv = createEmptyInvoice();
    try {
      const generatedXml = await invoke<string>('generate_zugferd_xml', { invoice: newInv });
      detection.has_zugferd = true;
      detection.filename = 'factur-x.xml';
      detection.invoice = newInv;
      detection.raw_xml = generatedXml;
      detection.xml_size_bytes = new TextEncoder().encode(generatedXml).length;
      isDirty = true;
    } catch (e) {
      errorMessage = `Fehler beim Erzeugen: ${e}`;
    }
  }

  // Option B: Externe XML-Datei von der Festplatte importieren
  async function handleImportXmlFile() {
    if (!detection) return;

    try {
      const filePath = await open({
        multiple: false,
        directory: false,
        title: 'ZUGFeRD/Factur-X XML-Datei auswählen',
        filters: [{ name: 'XML Rechnung', extensions: ['xml'] }],
      });

      if (typeof filePath !== 'string') return;

      // Datei als Bytes über unseren bestehenden Rust-Command lesen
      const rawBytes = await invoke<number[]>('read_pdf_bytes', { path: filePath });
      const xmlString = new TextDecoder('utf-8').decode(new Uint8Array(rawBytes));

      // In Rust parsen
      const parsedInvoice = await invoke<ZugferdInvoice>('parse_external_xml', {
        xmlContent: xmlString,
      });

      detection.has_zugferd = true;
      detection.filename = 'factur-x.xml';
      detection.invoice = parsedInvoice;
      detection.raw_xml = xmlString;
      detection.xml_size_bytes = rawBytes.length;
      isDirty = true;
    } catch (e) {
      errorMessage = `Fehler beim Importieren der XML: ${e}`;
    }
  }

  async function loadAndAnalyze(path: string) {
    isProcessing = true;
    errorMessage = null;
    isDirty = false;

    try {
      const [detectionRes, rawBytes] = await Promise.all([
        invoke<ZugferdDetection>('check_zugferd', { path }),
        invoke<number[]>('read_pdf_bytes', { path }),
      ]);

      // Jedem geladenen Item eine ID für Svelte 5 zuweisen
      if (detectionRes.invoice?.items) {
        detectionRes.invoice.items = detectionRes.invoice.items.map((it: InvoiceItem, idx: number) => ({
          ...it,
          id: `item-${idx}`,
        }));
      }

      detection = detectionRes;
      pdfBytes = new Uint8Array(rawBytes);
    } catch (err) {
      errorMessage = String(err);
    } finally {
      isProcessing = false;
    }
  }

  async function handleSaveUpdatedInvoice(updatedInvoice: ZugferdInvoice) {
    if (!detection) return;

    try {
      // 1. Neues CII-XML in Rust generieren lassen
      const newXml = await invoke<string>('generate_zugferd_xml', {
        invoice: updatedInvoice,
      });

      // 2. Reaktiven Zustand mit den neuen XML-Bytes aktualisieren
      detection.invoice = updatedInvoice;
      detection.raw_xml = newXml;
      detection.xml_size_bytes = new TextEncoder().encode(newXml).length;
      isDirty = true;
    } catch (err) {
      console.error('Fehler beim Generieren der ZUGFeRD-XML:', err);
    }
  }

  function handleReset() {
    selectedPdfPath = null;
    pdfBytes = null;
    detection = null;
    errorMessage = null;
    isDirty = false;
  }

  async function handleSavePdf() {
    if (!selectedPdfPath || !detection?.raw_xml) return;

    try {
      const targetPath = await save({
        title: 'Modifizierte ZUGFeRD-Rechnung speichern',
        defaultPath: fileName ? `zugferd_${fileName}` : 'rechnung_zugferd.pdf',
        filters: [{ name: 'PDF Dokument', extensions: ['pdf'] }],
      });

      if (!targetPath) return;

      isSaving = true;
      saveSuccessMessage = null;

      await invoke('save_zugferd_pdf', {
        sourcePath: selectedPdfPath,
        targetPath: targetPath,
        xmlContent: detection.raw_xml,
        profile: detection.invoice?.profile ?? 'EN 16931 (COMFORT)',
      });

      isDirty = false;
      saveSuccessMessage = 'PDF erfolgreich mit ZUGFeRD-Daten gespeichert!';
      
      // Neu laden, damit der Viewer den neuen Stand zeigt
      selectedPdfPath = targetPath;
      await loadAndAnalyze(targetPath);
    } catch (err) {
      errorMessage = `Fehler beim Speichern: ${err}`;
    } finally {
      isSaving = false;
    }
  }

  async function handleStripZugferd() {
    if (!selectedPdfPath || !detection?.has_zugferd) return;

    try {
      // 1. Zielort für das bereinigte Dokument wählen
      const targetPath = await save({
        title: 'Bereinigtes PDF ohne ZUGFeRD speichern unter',
        defaultPath: fileName ? `ohne_zugferd_${fileName}` : 'rechnung_ohne_zugferd.pdf',
        filters: [{ name: 'PDF Dokument', extensions: ['pdf'] }],
      });

      if (!targetPath) return;

      isStripping = true;
      errorMessage = null;

      // 2. Rust löscht Anhänge, Katalog-Einträge und XMP-Schemas
      await invoke('strip_zugferd', {
        sourcePath: selectedPdfPath,
        targetPath: targetPath,
      });

      // 3. Das frisch bereinigte Dokument direkt im Viewer öffnen
      selectedPdfPath = targetPath;
      await loadAndAnalyze(targetPath);
    } catch (err) {
      errorMessage = `Fehler beim Entfernen von ZUGFeRD: ${err}`;
    } finally {
      isStripping = false;
    }
  }
</script>

<div class="app-layout">
  <header class="top-bar">
    <div class="brand">
      <strong>ZUGFeRD Studio</strong>
      {#if fileName}
        <span class="file-tag">{fileName}</span>
      {/if}
      {#if isDirty}
        <span class="dirty-badge">Geändert</span>
      {/if}
    </div>

    <div class="actions">
      <!-- Info- & Onboarding-Button -->
      <button
        class="btn-subtle"
        onclick={() => (showAboutModal = true)}
        title="Über ZUGFeRD Studio & Hilfe"
      >
        ?
      </button>

      {#if selectedPdfPath}
        <button class="btn-subtle" onclick={handleReset}>Schließen</button>
        {#if detection?.has_zugferd}
          <button
            class="btn-save"
            onclick={handleSavePdf}
            disabled={isSaving}
          >
            {isSaving ? 'Wird gespeichert...' : 'PDF speichern unter...'}
          </button>
        {/if}
      {/if}
      <button class="btn-primary" onclick={handlePickFile}>
        {selectedPdfPath ? 'Anderes PDF wählen' : 'PDF öffnen'}
      </button>
    </div>
  </header>

  <main class="content-body">
    {#if !selectedPdfPath}
      <div class="center-splash">
        <h2>Kein Dokument geöffnet</h2>
        <p>Wähle ein PDF aus, um es anzuzeigen und ZUGFeRD-Daten zu editieren.</p>
        <button class="btn-primary" onclick={handlePickFile}>Datei auswählen</button>
      </div>
    {:else if isProcessing}
      <div class="center-splash">
        <p>Rust parst PDF und analysiert XML-Struktur...</p>
      </div>
    {:else}
      <div class="workspace-grid">
        <section class="viewer-pane">
          {#if pdfBytes}
            <PdfViewer pdfData={pdfBytes} />
          {/if}
        </section>

        <aside class="inspector-pane">
          {#if detection?.has_zugferd && detection.invoice}
            <ZugferdEditor
              invoice={detection.invoice}
              onSave={handleSaveUpdatedInvoice}
            />

            <div class="danger-zone">
              <button
                class="btn-danger full-width"
                onclick={handleStripZugferd}
                disabled={isStripping}
              >
                {isStripping ? 'Wird bereinigt...' : 'ZUGFeRD-Anhang entfernen'}
              </button>
            </div>
          {:else}
            <!-- NEU: Interaktives Upgrade-Panel für Standard-PDFs -->
            <div class="attach-workflow-card">
              <div class="attach-badge">Standard-Dokument</div>
              <h3>ZUGFeRD hinzufügen</h3>
              <p class="attach-intro">
                Dieses PDF enthält noch keine maschinenlesbaren Rechnungsdaten. Wandle es jetzt in eine E-Rechnung nach PDF/A-3 um:
              </p>

              <div class="attach-actions">
                <button class="btn-primary full-width" onclick={handleCreateNewInvoice}>
                  + Neue Rechnung erfassen
                </button>
                <div class="or-separator"><span>oder</span></div>
                <button class="btn-subtle full-width" onclick={handleImportXmlFile}>
                  Bestehende XML importieren...
                </button>
              </div>
            </div>
          {/if}
        </aside>
      </div>
    {/if}
  </main>

  <!-- Fullscreen Drop-Overlay beim Hovern von Dateien -->
  {#if isDraggingOver}
    <div class="drop-overlay">
      <div class="drop-target-card">
        <div class="drop-icon">📂</div>
        <h2>Datei hier ablegen</h2>
        <p>PDF zum Anzeigen/Bearbeiten oder XML zum Importieren hineinziehen</p>
      </div>
    </div>
  {/if}

  <AboutModal
    isOpen={showAboutModal}
    onClose={() => (showAboutModal = false)}
  />
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background-color: #0c0c0e;
    color: #e4e4e7;
    height: 100vh;
    overflow: hidden;
  }

  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .top-bar {
    height: 48px;
    background-color: #18181b;
    border-bottom: 1px solid #27272a;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.25rem;
  }

  .btn-save {
    background-color: #059669;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.45rem 0.9rem;
    font-size: 0.85rem;
    cursor: pointer;
    font-weight: 500;
    transition: background-color 0.15s;
  }

  .btn-save:hover:not(:disabled) {
    background-color: #047857;
  }

  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .file-tag {
    background-color: #27272a;
    font-size: 0.8rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    color: #a1a1aa;
  }

  .dirty-badge {
    background-color: #78350f;
    color: #fde68a;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .content-body {
    flex: 1;
    overflow: hidden;
  }

  .center-splash {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 1rem;
    text-align: center;
  }

  .workspace-grid {
    display: grid;
    grid-template-columns: 1fr 380px;
    height: 100%;
    overflow: hidden;
  }

  .viewer-pane {
    height: 100%;
    padding: 0.75rem;
    overflow: hidden;
  }

  .inspector-pane {
    background-color: #141417;
    border-left: 1px solid #27272a;
    padding: 1rem;
    overflow-y: auto;
  }

  .btn-primary {
    background-color: #4f46e5;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.45rem 0.9rem;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .btn-subtle {
    background: transparent;
    color: #a1a1aa;
    border: 1px solid #27272a;
    border-radius: 6px;
    padding: 0.45rem 0.8rem;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .danger-zone {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid #27272a;
  }

  .btn-danger {
    background-color: #3f1d1d;
    color: #fca5a5;
    border: 1px solid #7f1d1d;
    border-radius: 6px;
    padding: 0.5rem 0.8rem;
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.15s;
  }

  .btn-danger:hover:not(:disabled) {
    background-color: #551f1f;
  }

  .btn-danger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .full-width {
    width: 100%;
  }

  .attach-workflow-card {
    background-color: #1a1a20;
    border: 1px solid #2e2e38;
    border-radius: 8px;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .attach-badge {
    align-self: flex-start;
    background-color: #27272a;
    color: #a1a1aa;
    font-size: 0.7rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-weight: 600;
  }

  .attach-workflow-card h3 {
    margin: 0;
    font-size: 1.1rem;
    color: #fafafa;
  }

  .attach-intro {
    font-size: 0.8rem;
    color: #a1a1aa;
    line-height: 1.4;
    margin: 0;
  }

  .attach-actions {
    margin-top: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .or-separator {
    text-align: center;
    position: relative;
    margin: 0.25rem 0;
  }

  .or-separator::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    height: 1px;
    background-color: #27272a;
  }

  .or-separator span {
    position: relative;
    background-color: #1a1a20;
    padding: 0 0.5rem;
    font-size: 0.7rem;
    color: #71717a;
  }

  /* Fullscreen-Overlay für Drag-and-Drop */
  .drop-overlay {
    position: fixed;
    inset: 0;
    background-color: rgba(12, 12, 14, 0.85);
    backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    pointer-events: none; /* Verhindert Flickern bei Mausbewegungen */
  }

  .drop-target-card {
    border: 2px dashed #6366f1;
    background-color: #1e1e2d;
    padding: 3rem 4rem;
    border-radius: 12px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    animation: dropPulse 1.5s infinite alternate ease-in-out;
  }

  .drop-icon {
    font-size: 3rem;
  }

  .drop-target-card h2 {
    margin: 0;
    font-size: 1.4rem;
    color: #fafafa;
  }

  .drop-target-card p {
    margin: 0;
    font-size: 0.9rem;
    color: #a1a1aa;
  }

  @keyframes dropPulse {
    from {
      border-color: #4f46e5;
      transform: scale(0.99);
    }
    to {
      border-color: #818cf8;
      transform: scale(1.01);
    }
  }
</style>