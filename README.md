# ctx-gen

Genera un archivo de contexto con todo el código de tu proyecto, listo para pasarlo a un agente de IA.

## Instalación

Requiere [rustup](https://rustup.rs).

```bash
git clone <repo>
cd ctx-gen
cargo install --path .
```

## Uso

Ejecuta desde la raíz de cualquier proyecto:

```bash
ctx-gen
```

El output depende del tamaño del proyecto:

- **≤ 1000 líneas** → genera `context.md` en la raíz
- **> 1000 líneas** → genera `context.zip` con `context-1.md`, `context-2.md`, etc.

Si el proyecto crece o se reduce entre ejecuciones, el archivo del modo anterior se elimina automáticamente.

Cada parte incluye su propio encabezado. La parte 1 siempre contiene el árbol de archivos completo.

## Qué se incluye y qué no

**Respeta `.gitignore` automáticamente** — `node_modules/`, `target/`, `dist/` y cualquier cosa ignorada por git queda fuera.

**Excluye por defecto:**
- Archivos binarios e imágenes
- Lock files (`Cargo.lock`, `package-lock.json`, `yarn.lock`, `go.sum`, etc.)
- Archivos minificados (`*.min.js`, `*.min.css`)
- El propio `context.md` / `context.zip` y partes anteriores
- El directorio `.git/`

## .ctxignore

Para excluir archivos específicos del proyecto, crea un `.ctxignore` en la raíz con la misma sintaxis que `.gitignore`:

```gitignore
# Fixtures de test
tests/fixtures/
**/*.snap

# Código generado
src/generated/
proto/gen/
```

El `.ctxignore` se aplica en cascada igual que `.gitignore` — puedes poner uno en cualquier subdirectorio.
