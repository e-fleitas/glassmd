// src/main.ts

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles/globals.css";
import "./styles/components.css";

// ── Tipos ────────────────────────────────────────────────────────────────────
interface ConversionResult {
    markdown: string;
    title: string | null;
    duration_ms: number;
}

interface CommandError {
    message: string;
    error_type: string;
}

// ── Estado de la aplicación ──────────────────────────────────────────────────
// Un objeto central que representa qué está pasando en la app.
// En lugar de leer el DOM para saber el estado, leemos este objeto.
type AppState = "idle" | "loaded" | "loading" | "success" | "error";

const state = {
    current: "idle" as AppState,
    filePath: null as string | null,
};

// ── Bootstrap ────────────────────────────────────────────────────────────────
document.addEventListener("DOMContentLoaded", async () => {
    const app = document.getElementById("app")!;
    app.innerHTML = buildUI();

    await loadSupportedExtensions();
    setupDragAndDrop();
    setupExploreButton();
    setupConvertButton();
});

// ── Construcción del DOM ──────────────────────────────────────────────────────
function buildUI(): string {
    return `
    <div class="container">

      <header class="header">
        <h1 class="title">GlassMD</h1>
        <p class="subtitle">Turn any file into a Markdown</p>
      </header>

      <div id="drop-zone" class="drop-zone">
        <svg class="drop-icon" viewBox="0 0 48 48" fill="none"
             xmlns="http://www.w3.org/2000/svg">
          <path d="M24 8L24 32M24 8L16 16M24 8L32 16"
                stroke="currentColor" stroke-width="2.5"
                stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M8 36H40"
                stroke="currentColor" stroke-width="2.5"
                stroke-linecap="round"/>
        </svg>

        <p class="drop-hint" id="drop-hint">
          Drop files here
        </p>

        <p id="extensions-list" class="extensions-list">
          loading supported formats...
        </p>
      </div>

      <button id="btn-explore" class="btn-explore">
        Choose file or folder
      </button>

      <button id="btn-convert" class="btn-convert" disabled>
        Convert it!
      </button>

      <div id="status-area" class="status-area" style="display:none;">
        <p id="status-message"></p>
      </div>

      <div id="result-area" class="result-area" style="display:none;">
        <div class="result-header">
          <span id="result-title"></span>
          <span id="result-duration"></span>
        </div>
        <pre id="result-content" class="result-content"></pre>
      </div>

    </div>
  `;
}

// ── Gestión de estado ─────────────────────────────────────────────────────────
// Un solo lugar donde cambia el estado visual de la app.
// Nunca modifiques clases de estado desde otro lugar.
function setState(newState: AppState, message?: string): void {
    const dropZone = document.getElementById("drop-zone")!;
    const dropHint = document.getElementById("drop-hint")!;
    const btnConvert = document.getElementById("btn-convert") as HTMLButtonElement;
    const statusArea = document.getElementById("status-area")!;
    const statusMessage = document.getElementById("status-message")!;

    // Limpiamos todos los estados anteriores antes de aplicar el nuevo
    dropZone.classList.remove(
        "state-loaded", "state-loading", "state-success", "state-error"
    );

    state.current = newState;

    switch (newState) {
        case "idle":
            dropHint.textContent = "Drop a file or a folder";
            btnConvert.disabled = true;
            statusArea.style.display = "none";
            break;

        case "loaded":
            dropZone.classList.add("state-loaded");
            // Mostramos el nombre del archivo, no el path completo
            const fileName = state.filePath?.split("/").pop() ?? "";
            dropHint.textContent = `📄 ${fileName}`;
            btnConvert.disabled = false;
            statusArea.style.display = "none";
            break;

        case "loading":
            dropZone.classList.add("state-loading");
            dropHint.textContent = "Converting...";
            btnConvert.disabled = true;
            statusArea.style.display = "none";
            break;

        case "success":
            dropZone.classList.add("state-success");
            dropHint.textContent = "✓ File Converted";
            btnConvert.disabled = false;
            statusArea.style.display = "none";
            break;

        case "error":
            dropZone.classList.add("state-loaded"); // salmón para error también
            dropHint.textContent = "✕ Error during convertion";
            btnConvert.disabled = false;
            if (message) {
                statusArea.style.display = "block";
                statusMessage.textContent = message;
            }
            break;
    }
}

// ── Comunicación con Rust ─────────────────────────────────────────────────────
async function loadSupportedExtensions(): Promise<void> {
    try {
        const extensions = await invoke<string[]>("get_supported_extensions");
        const el = document.getElementById("extensions-list")!;
        el.textContent = extensions.map(e => `.${e}`).join("  ");
    } catch (err) {
        const el = document.getElementById("extensions-list")!;
        el.textContent = "⚠ Backend conection error";
        console.error("[GlassMD] Error:", err);
    }
}

async function convertFile(): Promise<void> {
    if (!state.filePath) return;

    setState("loading");

    try {
        const result = await invoke<ConversionResult>("convert_file", {
            filePath: state.filePath,
        });

        setState("success");
        showResult(result);

    } catch (err) {
        const error = err as CommandError;
        setState("error", error.message ?? String(err));
        console.error("[GlassMD] Convertion Error:", error);
    }
}

// ── Resultado ─────────────────────────────────────────────────────────────────
function showResult(result: ConversionResult): void {
    const resultArea = document.getElementById("result-area")!;
    const resultTitle = document.getElementById("result-title")!;
    const resultDuration = document.getElementById("result-duration")!;
    const resultContent = document.getElementById("result-content")!;

    const fileName = state.filePath?.split("/").pop() ?? "";
    resultTitle.textContent = result.title ?? fileName;
    resultDuration.textContent = `${result.duration_ms}ms`;
    resultContent.textContent = result.markdown;

    resultArea.style.display = "flex";
}

// ── Drag & Drop ───────────────────────────────────────────────────────────────
// En Tauri v2 el WebView no recibe eventos drop del DOM estándar.
// Tauri los intercepta a nivel OS y los re-emite como eventos propios.
function setupDragAndDrop(): void {
    const dropZone = document.getElementById("drop-zone")!;

    // Estilos visuales durante el drag — estos sí funcionan con DOM estándar
    dropZone.addEventListener("dragover", (e) => {
        e.preventDefault();
        dropZone.classList.add("drag-over");
    });

    dropZone.addEventListener("dragleave", () => {
        dropZone.classList.remove("drag-over");
    });

    // Escuchamos el evento nativo de Tauri en lugar del DOM drop

    getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type === "over") {
            dropZone.classList.add("drag-over");
        }

        if (event.payload.type === "leave") {
            dropZone.classList.remove("drag-over");
        }

        if (event.payload.type === "drop") {
            dropZone.classList.remove("drag-over");

            const paths = event.payload.paths;
            if (!paths || paths.length === 0) return;

            // Tomamos el primer path — batch conversion viene después
            state.filePath = paths[0];
            setState("loaded");
            console.log("[GlassMD] Archivo recibido:", paths[0]);
        }
    });
}

// ── Botón Explorar ────────────────────────────────────────────────────────────
function setupExploreButton(): void {
    const btn = document.getElementById("btn-explore")!;

    btn.addEventListener("click", async () => {
        // Abre el diálogo nativo del OS para seleccionar un archivo.
        // Los filtros limitan qué archivos ve el usuario — misma lógica
        // que la whitelist de security.rs pero en la UI.
        const selected = await open({
            multiple: false,
            filters: [{
                name: "Documentos soportados",
                extensions: [
                    "pdf", "docx", "pptx", "xlsx", "xls", "epub",
                    "html", "htm", "csv", "json", "xml",
                    "jpg", "jpeg", "png", "gif",
                    "md", "txt", "ipynb", "zip", "msg",
                ],
            }],
        });

        // `open()` devuelve null si el usuario canceló
        if (!selected) return;

        state.filePath = selected as string;
        setState("loaded");
    });
}

// ── Botón Convertir ───────────────────────────────────────────────────────────
function setupConvertButton(): void {
    const btn = document.getElementById("btn-convert")!;
    btn.addEventListener("click", () => convertFile());
}