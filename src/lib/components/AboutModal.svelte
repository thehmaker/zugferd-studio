<script lang="ts">
    import { openUrl } from '@tauri-apps/plugin-opener';

    interface Props {
        isOpen: boolean;
        onClose: () => void;
    }

    let { isOpen, onClose }: Props = $props();

    async function openGithub() {
        try {
        // Öffnet den Standardbrowser des Betriebssystems
        await openUrl('https://github.com/thehmaker/zugferd-studio');
        } catch {
        window.open('https://github.com/thehmaker/zugferd-studio', '_blank');
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') onClose();
    }
    </script>

    <svelte:window onkeydown={handleKeydown} />

    {#if isOpen}
    <div class="modal-backdrop" onclick={onClose} role="presentation">
        <div
            class="modal-box"
            onclick={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            >
            <div class="modal-header">
                <div class="app-identity">
                <div class="logo-circle">⚡</div>
                <div>
                    <h3>ZUGFeRD Studio</h3>
                    <span class="version">Version 1.0.0 (Open Source)</span>
                </div>
                </div>
                <button class="btn-close" onclick={onClose} title="Schließen">✕</button>
            </div>

            <div class="modal-body">
                <section class="onboarding-guide">
                <h4>Was kann diese App?</h4>
                <ul class="features-list">
                    <li>
                    <strong>PDFs betrachten:</strong> Dokumente visuell prüfen, Seiten durchblättern und Text pixelgenau markieren oder kopieren.
                    </li>
                    <li>
                    <strong>ZUGFeRD / Factur-X inspizieren:</strong> Eingebettete Rechnungsdaten (CII) sofort auslesen, Positionen und Summen strukturiert erfassen.
                    </li>
                    <li>
                    <strong>Live-Validierung (EN 16931):</strong> Automatische Plausibilitätsprüfung aller Beträge, Steuern und Pflichtfelder in Echtzeit.
                    </li>
                    <li>
                    <strong>Konvertieren & Bereinigen:</strong> Normale Standard-PDFs zu PDF/A-3 Rechnungen aufwerten oder XML-Anhänge rückstandslos entfernen.
                    </li>
                </ul>
                </section>

                <section class="creator-box">
                <div class="creator-text">
                    <span>Entwickelt von <strong>Christian Hamacher / TheHMaker</strong></span>
                    <p>
                    Kostenlos und quelloffen bereitgestellt unter der <strong>MIT-Lizenz</strong>. Keine Telemetrie, keine Cloud-Pflicht – alle Berechnungen laufen zu 100 % lokal auf deinem Rechner.
                    </p>
                </div>
                <button class="btn-github" onclick={openGithub}>
                    GitHub Repository
                </button>
                </section>
            </div>

            <div class="modal-footer">
                <button class="btn-primary" onclick={onClose}>Verstanden & Loslegen</button>
            </div>
        </div>
    </div>
    {/if}

    <style>
    .modal-backdrop {
        position: fixed;
        inset: 0;
        background-color: rgba(0, 0, 0, 0.75);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
        padding: 1.5rem;
    }

    .modal-box {
        background-color: #18181b;
        border: 1px solid #27272a;
        border-radius: 12px;
        width: 100%;
        max-width: 520px;
        box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .modal-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1.25rem 1.5rem;
        border-bottom: 1px solid #27272a;
    }

    .app-identity {
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }

    .logo-circle {
        width: 36px;
        height: 36px;
        background: linear-gradient(135deg, #4f46e5, #059669);
        border-radius: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 1.2rem;
    }

    .app-identity h3 {
        margin: 0;
        font-size: 1.1rem;
        color: #fafafa;
    }

    .version {
        font-size: 0.75rem;
        color: #a1a1aa;
    }

    .btn-close {
        background: transparent;
        border: none;
        color: #71717a;
        font-size: 1.1rem;
        cursor: pointer;
        padding: 0.2rem 0.4rem;
    }

    .btn-close:hover {
        color: #fafafa;
    }

    .modal-body {
        padding: 1.25rem 1.5rem;
        display: flex;
        flex-direction: column;
        gap: 1.25rem;
        max-height: 70vh;
        overflow-y: auto;
    }

    h4 {
        margin: 0 0 0.5rem 0;
        font-size: 0.85rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: #71717a;
    }

    .features-list {
        margin: 0;
        padding-left: 1.2rem;
        display: flex;
        flex-direction: column;
        gap: 0.45rem;
        font-size: 0.85rem;
        color: #d4d4d8;
    }

    .features-list strong {
        color: #f4f4f5;
    }

    .creator-box {
        background-color: #121215;
        border: 1px solid #27272a;
        border-radius: 8px;
        padding: 1rem;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .creator-text span {
        font-size: 0.85rem;
        color: #e4e4e7;
    }

    .creator-text p {
        margin: 0.35rem 0 0 0;
        font-size: 0.75rem;
        color: #a1a1aa;
        line-height: 1.4;
    }

    .btn-github {
        align-self: flex-start;
        background-color: #27272a;
        color: #e4e4e7;
        border: 1px solid #3f3f46;
        border-radius: 6px;
        padding: 0.35rem 0.75rem;
        font-size: 0.8rem;
        cursor: pointer;
        transition: background 0.15s;
    }

    .btn-github:hover {
        background-color: #3f3f46;
    }

    .modal-footer {
        padding: 1rem 1.5rem;
        background-color: #141417;
        border-top: 1px solid #27272a;
        display: flex;
        justify-content: flex-end;
    }

    .btn-primary {
        background-color: #4f46e5;
        color: white;
        border: none;
        border-radius: 6px;
        padding: 0.5rem 1.1rem;
        font-size: 0.85rem;
        font-weight: 500;
        cursor: pointer;
    }

    .btn-primary:hover {
        background-color: #4338ca;
    }
</style>