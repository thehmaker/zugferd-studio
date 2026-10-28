<script lang="ts">
    import type * as pdfjs from 'pdfjs-dist';

    interface Props {
        pdfDoc: pdfjs.PDFDocumentProxy;
        pageNumber: number;
        isSelected: boolean;
        onClick: () => void;
    }

    let { pdfDoc, pageNumber, isSelected, onClick }: Props = $props();

    let canvasElement: HTMLCanvasElement | null = $state(null);
    let renderTask: pdfjs.RenderTask | null = null;

    $effect(() => {
        if (!pdfDoc || !canvasElement) return;

        let active = true;

        pdfDoc.getPage(pageNumber).then((page) => {
        if (!active || !canvasElement) return;

        const unscaled = page.getViewport({ scale: 1.0 });
        const targetWidth = 110;
        const thumbScale = targetWidth / unscaled.width;
        const viewport = page.getViewport({ scale: thumbScale });

        const pixelRatio = window.devicePixelRatio || 1;
        canvasElement.width = Math.floor(viewport.width * pixelRatio);
        canvasElement.height = Math.floor(viewport.height * pixelRatio);
        canvasElement.style.width = `${Math.floor(viewport.width)}px`;
        canvasElement.style.height = `${Math.floor(viewport.height)}px`;

        const ctx = canvasElement.getContext('2d');
        if (!ctx) return;

        if (renderTask) {
            renderTask.cancel();
        }

        renderTask = page.render({
            canvasContext: ctx,
            canvas: canvasElement,
            viewport,
            transform: pixelRatio !== 1 ? [pixelRatio, 0, 0, pixelRatio, 0, 0] : undefined,
        });

        renderTask.promise.catch((err: any) => {
            if (err?.name !== 'RenderingCancelledException') {
            console.error(`Thumbnail #${pageNumber} Render-Fehler:`, err);
            }
        });
        });

        return () => {
        active = false;
        if (renderTask) {
            renderTask.cancel();
            renderTask = null;
        }
        };
    });
    </script>

    <div
    class="thumb-item"
    class:selected={isSelected}
    onclick={onClick}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Enter' && onClick()}
    >
    <div class="canvas-wrapper">
        <canvas bind:this={canvasElement}></canvas>
    </div>
    <span class="page-label">Seite {pageNumber}</span>
    </div>

    <style>
    .thumb-item {
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 0.5rem;
        border-radius: 6px;
        cursor: pointer;
        background-color: #18181c;
        border: 2px solid transparent;
        transition: all 0.15s ease;
        user-select: none;
        margin-bottom: 0.6rem;
    }

    .thumb-item:hover {
        background-color: #22222a;
        border-color: #3f3f46;
    }

    .thumb-item.selected {
        background-color: #1e1e2d;
        border-color: #6366f1;
    }

    .canvas-wrapper {
        background-color: #ffffff;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
        display: flex;
        justify-content: center;
        align-items: center;
        border-radius: 2px;
        overflow: hidden;
    }

    canvas {
        display: block;
    }

    .page-label {
        font-size: 0.7rem;
        color: #a1a1aa;
        margin-top: 0.35rem;
    }

    .thumb-item.selected .page-label {
        color: #c7d2fe;
        font-weight: 600;
    }
</style>