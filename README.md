# GlassMD ── Cyber-Minimalista MarkItDown Desktop GUI

[![Licencia](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/built%20with-Tauri%20v2-8a2be2.svg)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/backend-Rust-orange.svg)](https://www.rust-lang.org/)

**GlassMD** es una interfaz gráfica (GUI) de escritorio, ultra-segura, veloz y con una estética ciber-minimalista diseñada para ejecutar de forma 100% local la potente herramienta de conversión de archivos [MarkItDown](https://github.com/microsoft/markitdown) de Microsoft.

El proyecto está construido utilizando **Tauri (v2) y Rust** para el empaquetado del sistema y la seguridad de memoria, integrando un entorno de **Python embebido (Sidecar)** para garantizar una experiencia totalmente autónoma e independiente de dependencias externas (*out-of-the-box*).

---

## 🚀 La Motivación del Proyecto

El ecosistema de conversión de formatos a Markdown suele requerir flujos de trabajo repetitivos en la terminal, configuración manual de entornos virtuales de Python (`venv`), instalación de gestores de paquetes (`pip`) y lidiar con variables de entorno del sistema operativo (un problema crítico e intimidante para usuarios en entornos Windows).

**GlassMD resuelve esta fricción eliminando la barrera técnica:**
*   **Cero Configuración:** El usuario final solo descarga el instalador nativo (`.exe` o `.deb`) y la herramienta funciona de inmediato, sin requerir Python instalado en el sistema operativo anfitrión.
*   **Enfoque UI/UX Visual:** Sustituye los comandos largos de la CLI por una interfaz fluida con áreas de arrastrar y soltar (*Drop Zones*) y selectores de archivos nativos del sistema.
*   **Privacidad Absoluta (Security-First):** Todo el procesamiento de documentos (PDF, Office, imágenes, etc.) se realiza de manera estrictamente local en la máquina del usuario, aislando el frontend web de accesos no autorizados al sistema mediante el backend seguro de Rust.

---

## 🛠️ Arquitectura y Stack Tecnológico

Para garantizar la máxima performance y protección contra vectores de ataque, la aplicación se divide en tres capas desacopladas:

1.  **Frontend (Cyber-Minimalism UI):** Construido con tecnologías web modernas, TypeScript y Tailwind CSS. Implementa un diseño translúcido adaptativo (*Mica/Acrylic* en Windows y *Vibrancy* en Linux) enfocado en micro-animaciones dinámicas y paneles de telemetría de procesamiento en tiempo real.
2.  **Core de Seguridad (Tauri + Rust):** Actúa como el puente del sistema. Se encarga de la validación estricta de rutas de archivos para mitigar ataques de *Path Traversal* y maneja la comunicación asíncrona segura hacia el backend.
3.  **Motor de Conversión (Python Sidecar):** Ejecuta de forma nativa e invisible la librería original de **[Microsoft MarkItDown](https://github.com/microsoft/markitdown)**, aislando sus dependencias en un entorno embebido dentro del propio binario compilado.