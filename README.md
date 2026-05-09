# pageseer

Rasterize documents into per-page images.

`pageseer` is a Rust library and CLI that converts PDF, Office, and HWP/HWPX files into per-page PNG or JPEG images. It is intended as a preprocessing step for pipelines that operate on page images — OCR, vision-language models, search indexing.

## Scope

**Supported inputs:** PDF, DOCX/DOC, XLSX/XLS, PPTX/PPT, ODT/ODS/ODP, RTF, HWP/HWPX

**Output:** PNG or JPEG, one file per page

**Platforms:** Linux x86_64, Windows x86_64, macOS Apple Silicon

**Not in scope:**
- Page-range selection, embedded image extraction
- VLM/OCR adapters, streaming API
- Authenticated Gotenberg, static pdfium linking
- crates.io publication (blocked on the `rhwp` git dependency)

## Pipeline

```
PDF  ─────────────────────────────────────────────────────┐
Office → Gotenberg (LibreOffice) ─┐                        │
HWP    → rhwp (HWP → SVG → PDF) ──┤                        │
                                  ▼                        ▼
                                  PDF ──▶ pdfium-render ──▶ PNG/JPEG
```

All inputs are normalized to PDF before rasterization. Gotenberg is required only for Office formats; PDF and HWP processing has no external service dependency.

## Requirements

- Rust 1.75 or newer
- A `pdfium` shared library (dynamically loaded). Download a build for your platform from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) and place it at `<repo>/pdfium/` or on the system library search path:
  - Linux: `libpdfium.so`
  - Windows: `pdfium.dll`
  - macOS: `libpdfium.dylib`
- Gotenberg, only for Office formats: `docker run --rm -p 3000:3000 gotenberg/gotenberg:8`
- CJK fonts on the host when processing HWP/HWPX (Noto Sans CJK on Linux; Windows and macOS ship suitable fonts by default)

## Install

Pre-built binaries are attached to each [GitHub release](https://github.com/iyulab/pageseer/releases). Each archive includes the matching `pdfium` shared library.

Build from source:

```sh
cargo build --release
./target/release/pageseer --help
```

## CLI

```sh
pageseer <INPUT> [OPTIONS]
```

| Flag | Default | Description |
|---|---|---|
| `-o, --output <DIR>` | `./out` | Output directory |
| `-f, --format <FMT>` | `png` | `png` or `jpeg` |
| `--dpi <N>` | `150` | Rasterization DPI |
| `-q, --quality <1-100>` | `85` | JPEG quality (ignored for PNG) |
| `--max-edge <N>` | unset | Downscale so the long edge does not exceed N pixels (Lanczos3) |
| `--flat` | off | Flat layout: `<out>/<stem>-NNN.<ext>` instead of `<out>/<stem>/page-NNN.<ext>` |
| `--strict` | off | Stop on first failure (default: continue-on-error) |
| `--gotenberg-url <URL>` | `http://localhost:3000` | Gotenberg base URL (also `GOTENBERG_URL`) |
| `--gotenberg-timeout <SEC>` | `120` | Gotenberg request timeout |

Examples:

```sh
pageseer report.pdf --dpi 200
pageseer report.docx --format jpeg --quality 80 -o ./out
pageseer deck.pptx --max-edge 2048
pageseer slides.pdf --flat -o ./out
pageseer doc.docx --gotenberg-url http://gotenberg.internal:3000
```

Exit codes: `0` success, `1` total failure, `2` partial failure, `64` invalid arguments or unsupported format.

On partial failure, `<output>/<stem>/errors.json` is written with 1-based page numbers and stage identifiers (`source-read`, `convert`, `rasterize`, `write`).

## Library

```rust
use pageseer::{extract, ImageFormat, Options, SourceInput};

let report = extract(
    SourceInput::Path("report.pdf".into()),
    Options { format: ImageFormat::Png, dpi: 200, ..Options::default() },
)?;

println!("{} pages, {} failed", report.succeeded_count(), report.failed_count());
```

`extract` is synchronous. Partial failures are returned as `PageseerError::Partial(report)` rather than discarded.

> **Note:** HWP processing may panic inside `rhwp` on malformed input. Callers that need isolation should wrap the call in `std::panic::catch_unwind`.

## Testing

Unit tests (no external dependencies):

```sh
cargo test
```

Integration tests require `pdfium` and are gated behind `#[ignore]`:

```sh
cargo test -- --include-ignored
```

The Office integration test additionally requires a running Gotenberg server at `PAGESEER_TEST_GOTENBERG_URL`. The HWP integration test requires `tests/fixtures/sample.hwp` to be supplied by the user (CI does not run it).

## License

[MIT](./LICENSE)
