# sidecar/markitdown_sidecar.py
#
# Sidecar de Python para GlassMD.
# Recibe un JSON por argv[1], convierte el archivo con MarkItDown,
# imprime un JSON de respuesta en stdout y termina.
#
# CONTRATO CON RUST (sidecar.rs):
#   Input : argv[1] = '{"file_path": "...", "options": {...}}'
#   Output: stdout  = '{"status": "success", "markdown": "...", "title": "..."}'
#                   = '{"status": "error",   "error": "...", "error_type": "..."}'
#
# REGLA DE ORO: este script NUNCA debe imprimir nada a stdout excepto
# el JSON final. Cualquier print de debug usa sys.stderr.

import sys
import json


def main() -> None:
    # ── 1. Leer y parsear el argumento ──────────────────────────────────────
    # Rust nos pasa el JSON como primer argumento de línea de comandos.
    # argv[0] = ruta del propio script/ejecutable (como en todo programa)
    # argv[1] = nuestro JSON de request
    if len(sys.argv) < 2:
        _respond_error(
            "No se recibió argumento de request",
            "MissingArgument"
        )
        return

    try:
        request = json.loads(sys.argv[1])
    except json.JSONDecodeError as e:
        _respond_error(
            f"JSON de request inválido: {e}",
            "InvalidRequest"
        )
        return

    # ── 2. Extraer y validar el file_path ───────────────────────────────────
    # Rust ya validó este path antes de mandárnoslo, pero una segunda
    # verificación cuesta poco y nos da mejores mensajes de error.
    file_path: str = request.get("file_path", "")
    if not file_path:
        _respond_error("El campo 'file_path' está vacío", "InvalidRequest")
        return

    # ── 3. Importar MarkItDown ───────────────────────────────────────────────
    # El import va aquí adentro (no al tope del archivo) por una razón:
    # si MarkItDown no está instalado, el error queda capturado por el
    # try/except de main() y se convierte en un JSON de error legible.
    # Un ImportError al tope del script mataría el proceso silenciosamente.
    try:
        from markitdown import MarkItDown
    except ImportError:
        _respond_error(
            "MarkItDown no está instalado en el entorno Python embebido. "
            "Reconstruye el sidecar con: pip install markitdown",
            "SidecarNotBuilt"
        )
        return

    # ── 4. Convertir ────────────────────────────────────────────────────────
    try:
        md = MarkItDown()
        result = md.convert(file_path)

        # result.text_content  → el Markdown generado (str)
        # result.title         → título del documento si está disponible (str | None)
        markdown_text: str = result.text_content or ""
        title: str | None = getattr(result, "title", None)

        _respond_success(markdown_text, title)

    except FileNotFoundError:
        _respond_error(
            f"Archivo no encontrado: {file_path}",
            "FileNotFound"
        )
    except PermissionError:
        _respond_error(
            f"Sin permisos para leer: {file_path}",
            "PermissionError"
        )
    except Exception as e:
        # Captura cualquier error interno de MarkItDown (formato corrupto,
        # dependencia faltante para ese tipo de archivo, etc.)
        _respond_error(
            f"Error durante la conversión: {type(e).__name__}: {e}",
            "ConversionError"
        )


# ── Helpers de respuesta ──────────────────────────────────────────────────────
# Estas funciones garantizan que el JSON de salida siempre tenga
# la misma forma. Rust depende de esta consistencia.

def _respond_success(markdown: str, title: str | None) -> None:
    """Imprime la respuesta de éxito y termina."""
    response = {
        "status": "success",
        "markdown": markdown,
        "title": title,
    }
    # ensure_ascii=False preserva caracteres UTF-8 (tildes, ñ, etc.)
    print(json.dumps(response, ensure_ascii=False))


def _respond_error(message: str, error_type: str) -> None:
    """Imprime la respuesta de error y termina."""
    response = {
        "status": "error",
        "error": message,
        "error_type": error_type,
    }
    print(json.dumps(response, ensure_ascii=False))
    # También logueamos a stderr para que sea visible en desarrollo
    print(f"[sidecar error] {error_type}: {message}", file=sys.stderr)


# ── Entry point ───────────────────────────────────────────────────────────────
# Este patrón (if __name__ == "__main__") es estándar en Python.
# Permite importar este archivo en tests sin ejecutar main().
if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        # Última red de seguridad: si main() explota por algo que no
        # anticipamos, aún así imprimimos un JSON válido.
        _respond_error(
            f"Error fatal del sidecar: {type(e).__name__}: {e}",
            "FatalError"
        )
        sys.exit(1)