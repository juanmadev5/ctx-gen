# ctx-gen

Genera un único archivo `context.md` con todo el código de tu proyecto, listo para pasarlo a un agente de IA.

## Instalación

```bash
git clone <repo>
cd ctx-gen
cargo install --path .
```

Requiere Rust 1.75+.

## Uso

Ejecuta desde la raíz de cualquier proyecto:

```bash
ctx-gen
```

Genera `context.md` en el directorio actual con:

- Un árbol de archivos del proyecto
- El contenido de cada archivo en bloques de código Markdown con el lenguaje correspondiente

## Qué se incluye y qué no

**Respeta `.gitignore` automáticamente** — `node_modules/`, `target/`, `dist/` y cualquier cosa ignorada por git queda fuera.

**Excluye por defecto:**
- Archivos binarios e imágenes
- Lock files (`Cargo.lock`, `package-lock.json`, `yarn.lock`, `go.sum`, etc.)
- El propio `context.md` (para no entrar en bucle)
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
