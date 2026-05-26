// src/main.ts
//
// Punto de entrada del frontend de GlassMD.
// Este archivo es el único responsable de:
//   1. Construir el DOM inicial
//   2. Verificar comunicación con el backend Rust
//   3. Configurar drag & drop
//
// NOTA: Este es el frontend MÍNIMO para verificar que todo el stack
// funciona. El diseño visual final viene en un hito posterior.

import { invoke } from "@tauri-apps/api/core";

// ── Tipos ────────────────────────────────────────────────────────────────────
// Espejo del struct ConversionResult definido en commands.rs.
// TypeScript no puede leer Rust directamente — mantenemos estos tipos
// sincronizados manualmente. Si cambias el struct en Rust, actualiza aquí.
interface ConversionResult {
    markdown: string;
    title: string | null;
    duration_ms: number;
}

// Espejo de CommandError de commands.rs
interface CommandError {
    message: string;
    error_type: string;
}

// ── Bootstrap ────────────────────────────────────────────────────────────────
// Esperamos a que el DOM esté listo antes de manipularlo.
// Es el equivalente moderno del viejo window.onload.
document.addEventListener("DOMContentLoaded", async () => {
    // Construimos el HTML de la app e lo inyectamos en #app
    const app = document.getElementById("app")!;
    app.innerHTML = buildUI();

    // Verificamos comunicación con Rust al arrancar.
    // Si esto falla, hay un problema de configuración fundamental.
    await loadSupportedExtensions();

    // Activamos el drag & drop sobre toda la ventana
    setupDragAndDrop();
});

// ── Construcción del DOM ──────────────────────────────────────────────────────
// Retorna el HTML inicial como string.
// Usamos IDs específicos para luego referenciar los elementos con getElementById.
function buildUI(): string {
    return `
    <div class="container">

      <header class="header">
        <h1 class="title">GlassMD</h1>
        <p class="subtitle">Convierte cualquier documento a Markdown</p>
      </header>

      <main>
        <!-- Zona de drop -->
        <div id="drop-zone" class="drop-zone">
          <p class="drop-hint">Arrastra un archivo aquí</p>
          <p id="extensions-list" class="extensions-list">
            Cargando formatos soportados...
          </p>
        </div>

        <!-- Estado / resultado -->
        <div id="status-area" class="status-area" style="display:none;">
          <p id="status-message"></p>
        </div>

        <!-- Área de resultado Markdown (cruda por ahora) -->
        <div id="result-area" class="result-area" style="display:none;">
          <div class="result-header">
            <span id="result-title"></span>
            <span id="result-duration"></span>
          </div>
          <pre id="result-content"></pre>
        </div>
      </main>

    </div>
  `;
}

// ── Comunicación con Rust ─────────────────────────────────────────────────────

// Llama al comando Rust get_supported_extensions y muestra la lista en el UI.
// Este es el primer invoke() real — si funciona, Rust y el frontend se hablan.
async function loadSupportedExtensions(): Promise<void> {
    try {
        // invoke<string[]> le dice a TypeScript que esperamos un array de strings.
        // Rust tiene registrado este comando en lib.rs → commands::get_supported_extensions
        const extensions = await invoke<string[]>("get_supported_extensions");

        const el = document.getElementById("extensions-list")!;
        el.textContent = extensions.map(e => `.${e}`).join("  ");

        console.log("[GlassMD] Backend Rust respondió correctamente.", extensions);
    } catch (err) {
        // Si falla, mostramos el error en lugar de las extensiones.
        // Esto nos ayuda a diagnosticar problemas de configuración.
        const el = document.getElementById("extensions-list")!;
        el.textContent = "⚠ Error al conectar con el backend";
        console.error("[GlassMD] Error invocando get_supported_extensions:", err);
    }
}

// Convierte un archivo llamando al comando Rust convert_file.
// Recibe el path absoluto del archivo (extraído del evento de drop).
async function convertFile(filePath: string): Promise<void> {
    const statusArea = document.getElementById("status-area")!;
    const statusMessage = document.getElementById("status-message")!;
    const resultArea = document.getElementById("result-area")!;

    // Mostramos estado de carga
    resultArea.style.display = "none";
    statusArea.style.display = "block";
    statusMessage.textContent = `Convirtiendo: ${filePath}...`;

    try {
        const result = await invoke<ConversionResult>("convert_file", {
            filePath,
        });

        // Éxito: mostramos el resultado
        statusArea.style.display = "none";
        showResult(result, filePath);

    } catch (err) {
        // El error que llega aquí es el CommandError serializado por Rust.
        // Lo casteamos para poder acceder a sus campos tipados.
        const error = err as CommandError;
        statusMessage.textContent = `Error: ${error.message ?? String(err)}`;
        console.error("[GlassMD] Error de conversión:", error);
    }
}

// ── Resultado ─────────────────────────────────────────────────────────────────

function showResult(result: ConversionResult, filePath: string): void {
    const resultArea = document.getElementById("result-area")!;
    const resultTitle = document.getElementById("result-title")!;
    const resultDuration = document.getElementById("result-duration")!;
    const resultContent = document.getElementById("result-content")!;

    // Título: usamos el del documento si existe, sino el nombre del archivo
    const fileName = filePath.split("/").pop() ?? filePath;
    resultTitle.textContent = result.title ?? fileName;
    resultDuration.textContent = `${result.duration_ms}ms`;
    resultContent.textContent = result.markdown;

    resultArea.style.display = "block";
}

// ── Drag & Drop ───────────────────────────────────────────────────────────────
// Configuramos los eventos de drag & drop sobre la ventana completa.
// Importante: debemos llamar preventDefault() en dragover para que
// el browser no abra el archivo con su comportamiento por defecto.

function setupDragAndDrop(): void {
    const dropZone = document.getElementById("drop-zone")!;

    // Necesario para que el drop funcione: sin esto el browser ignora el drop
    dropZone.addEventListener("dragover", (e) => {
        e.preventDefault();
        dropZone.classList.add("drag-over");
    });

    dropZone.addEventListener("dragleave", () => {
        dropZone.classList.remove("drag-over");
    });

    dropZone.addEventListener("drop", async (e) => {
        e.preventDefault();
        dropZone.classList.remove("drag-over");

        // Extraemos los archivos del evento de drop
        const files = e.dataTransfer?.files;
        if (!files || files.length === 0) return;

        // Por ahora procesamos solo el primer archivo.
        // La conversión batch (múltiples archivos) viene en un hito posterior.
        const file = files[0];

        // IMPORTANTE: en el WebView de Tauri, file.path contiene el path
        // absoluto del sistema de archivos. En un navegador normal esto
        // sería vacío por seguridad, pero Tauri lo expone intencionalmente.
        const filePath = (file as File & { path: string }).path;

        if (!filePath) {
            console.error("[GlassMD] No se pudo obtener el path del archivo.");
            return;
        }

        await convertFile(filePath);
    });
}