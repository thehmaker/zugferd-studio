<!-- src/lib/components/PdfViewer.svelte -->
<script lang="ts">
    import { onMount } from 'svelte';
    import * as pdfjs from 'pdfjs-dist';
    import workerUrl from 'pdfjs-dist/build/pdf.worker.mjs?url';
    import PdfThumbnail from './PdfThumbnail.svelte';

    interface Props {
        pdfData: Uint8Array;
    }
    let { pdfData }: Props = $props();

    let canvasElement: HTMLCanvasElement | null = $state(null);
    let textLayerElement: HTMLDivElement | null = $state(null);
    let pdfDoc = $state<pdfjs.PDFDocumentProxy | null>(null);

    let currentPage = $state(1);
    let totalPages = $state(0);
    let scale = $state(1.2);
    let showThumbnails = $state(true);

    // Exakte CSS-Größe für Canvas + TextLayer Wrapper
    let pageWidth = $state(0);
    let pageHeight = $state(0);

    let currentRenderTask: pdfjs.RenderTask | null = null;

    onMount(() => {
        pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;
    });

    // Dokument laden, wenn Bytes ankommen
    $effect(() => {
        if (!pdfData || pdfData.length === 0) return;

        let cancelled = false;
        const loadingTask = pdfjs.getDocument({ data: pdfData.slice() });

        loadingTask.promise
        .then((doc) => {
            if (cancelled) return;
            pdfDoc = doc;
            totalPages = doc.numPages;
            currentPage = 1;
        })
        .catch((err) => {
            console.error('Fehler beim Parsen der PDF-Bytes:', err);
        });

        return () => {
        cancelled = true;
        };
    });

    // Seite + TextLayer neu rendern, wenn sich Dokument, Seite oder Zoom ändern
    $effect(() => {
        if (!pdfDoc || !canvasElement || !textLayerElement) return;

        const doc = pdfDoc;
        const canvas = canvasElement;
        const textLayer = textLayerElement;
        const pageNum = currentPage;
        const currentScale = scale;

        if (currentRenderTask) {
        currentRenderTask.cancel();
        currentRenderTask = null;
        }

        renderPage(doc, canvas, textLayer, pageNum, currentScale);
    });

    async function renderPage(
        doc: pdfjs.PDFDocumentProxy,
        canvas: HTMLCanvasElement,
        textLayer: HTMLDivElement,
        pageNum: number,
        renderScale: number
    ) {
        try {
        const page = await doc.getPage(pageNum);
        const viewport = page.getViewport({ scale: renderScale });
        const context = canvas.getContext('2d');
        if (!context) return;

        const pixelRatio = window.devicePixelRatio || 1;

        // Dimensionen im State sichern
        pageWidth = Math.floor(viewport.width);
        pageHeight = Math.floor(viewport.height);

        // Canvas Puffer-Auflösung (HiDPI)
        canvas.width = Math.floor(viewport.width * pixelRatio);
        canvas.height = Math.floor(viewport.height * pixelRatio);
        canvas.style.width = `${pageWidth}px`;
        canvas.style.height = `${pageHeight}px`;

        // 1. Visuelles Rendering auf Canvas
        const renderTask = page.render({
            canvasContext: context,
            canvas: canvas,
            viewport,
            transform: pixelRatio !== 1 ? [pixelRatio, 0, 0, pixelRatio, 0, 0] : undefined,
        });

        currentRenderTask = renderTask;
        await renderTask.promise;

        // 2. Text-Layer synchron darüber legen
        textLayer.innerHTML = '';
        textLayer.style.width = `${pageWidth}px`;
        textLayer.style.height = `${pageHeight}px`;

        const textContent = await page.getTextContent();

        // Kompatibel mit modernen TextLayer-Klassen in pdfjs-dist
        if ('TextLayer' in pdfjs && typeof (pdfjs as any).TextLayer === 'function') {
            const tl = new (pdfjs as any).TextLayer({
            textContentSource: textContent,
            container: textLayer,
            viewport,
            });
            await tl.render();
        } else if ('renderTextLayer' in pdfjs) {
            await (pdfjs as any).renderTextLayer({
            textContentSource: textContent,
            textContent,
            container: textLayer,
            viewport,
            }).promise;
        }
        } catch (err: any) {
        if (err?.name !== 'RenderingCancelledException') {
            console.error('Render-Fehler:', err);
        }
        } finally {
        currentRenderTask = null;
        }
    }

    function nextPage() {
        if (currentPage < totalPages) currentPage++;
    }

    function prevPage() {
        if (currentPage > 1) currentPage--;
    }

    function zoomIn() {
        scale = Math.min(scale + 0.2, 3.0);
    }

    function zoomOut() {
        scale = Math.max(scale - 0.2, 0.5);
    }
    </script>

    <div class="viewer-wrapper">
    <!-- Obere Steuerleiste -->
    <div class="viewer-toolbar">
        <div class="nav-group">
        <button
            class="btn-tool"
            class:active={showThumbnails}
            onclick={() => (showThumbnails = !showThumbnails)}
            title="Seitenleiste ein-/ausblenden"
        >
            Seiten ({totalPages})
        </button>
        <div class="sep"></div>
        <button class="btn-tool" onclick={prevPage} disabled={currentPage <= 1}>Zurück</button>
        <span class="page-counter">{currentPage} / {totalPages}</span>
        <button class="btn-tool" onclick={nextPage} disabled={currentPage >= totalPages}>Weiter</button>
        </div>

        <div class="zoom-group">
        <button class="btn-tool" onclick={zoomOut}>-</button>
        <span class="zoom-label">{Math.round(scale * 100)}%</span>
        <button class="btn-tool" onclick={zoomIn}>+</button>
        </div>
    </div>

    <!-- Arbeitsbereich: Sidebar + Viewport -->
    <div class="viewer-body">
        {#if showThumbnails && pdfDoc && totalPages > 0}
        <aside class="thumbnail-sidebar">
            {#each Array(totalPages) as _, i (i + 1)}
            {@const pageNum = i + 1}
            <PdfThumbnail
                {pdfDoc}
                pageNumber={pageNum}
                isSelected={currentPage === pageNum}
                onClick={() => (currentPage = pageNum)}
            />
            {/each}
        </aside>
        {/if}

        <div class="canvas-scroll-container">
        <div
            class="page-wrapper"
            style="width: {pageWidth}px; height: {pageHeight}px; visibility: {pageWidth > 0 ? 'visible' : 'hidden'};"
        >
            <canvas bind:this={canvasElement}></canvas>
            <div class="textLayer" bind:this={textLayerElement}></div>
        </div>
        </div>
    </div>
    </div>

    <style>
    .viewer-wrapper {
        display: flex;
        flex-direction: column;
        height: 100%;
        background-color: #141417;
        border-radius: 8px;
        overflow: hidden;
        border: 1px solid #27272a;
    }

    .viewer-toolbar {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.45rem 0.8rem;
        background-color: #1c1c21;
        border-bottom: 1px solid #27272a;
        z-index: 10;
    }

    .nav-group,
    .zoom-group {
        display: flex;
        align-items: center;
        gap: 0.4rem;
    }

    .sep {
        width: 1px;
        height: 16px;
        background-color: #3f3f46;
        margin: 0 0.3rem;
    }

    .page-counter,
    .zoom-label {
        font-size: 0.8rem;
        color: #d4d4d8;
        min-width: 55px;
        text-align: center;
    }

    .btn-tool {
        background-color: #27272a;
        color: #e4e4e7;
        border: none;
        border-radius: 4px;
        padding: 0.3rem 0.65rem;
        font-size: 0.8rem;
        cursor: pointer;
        transition: background 0.15s;
    }

    .btn-tool:hover:not(:disabled) {
        background-color: #3f3f46;
    }

    .btn-tool.active {
        background-color: #4f46e5;
        color: white;
    }

    .btn-tool:disabled {
        opacity: 0.4;
        cursor: not-allowed;
    }

    .viewer-body {
        flex: 1;
        display: flex;
        overflow: hidden;
    }

    .thumbnail-sidebar {
        width: 145px;
        background-color: #121215;
        border-right: 1px solid #27272a;
        padding: 0.75rem 0.5rem;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
    }

    .canvas-scroll-container {
        flex: 1;
        overflow: auto;
        display: flex;
        justify-content: center;
        align-items: flex-start;
        padding: 1.5rem;
        background-color: #0c0c0e;
    }

    /* Exakter Schicht-Container für Canvas und TextLayer */
    .page-wrapper {
        position: relative;
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
        background-color: #ffffff;
        border-radius: 2px;
    }

    canvas {
        position: absolute;
        top: 0;
        left: 0;
        display: block;
    }

    /* Text-Layer für Textselektion */
    .textLayer {
        position: absolute;
        text-align: initial;
        top: 0;
        left: 0;
        overflow: hidden;
        line-height: 1;
        text-size-adjust: none;
        forced-color-adjust: none;
        transform-origin: 0 0;
        z-index: 2;
        user-select: text;
    }

    :global(.textLayer span),
    :global(.textLayer br) {
        color: transparent;
        position: absolute;
        white-space: pre;
        cursor: text;
        transform-origin: 0% 0%;
    }

    /* Sichtbare Markierung über dem Canvas */
    :global(.textLayer ::selection) {
        background: rgba(99, 102, 241, 0.4);
        color: transparent;
    }
</style>