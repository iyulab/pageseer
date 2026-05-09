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
pageseer <INPUT>... [OPTIONS]
```

One or more input files are accepted in a single invocation.

| Flag | Default | Description |
|---|---|---|
| `-o, --output <DIR>` | `./out` | Output directory |
| `-f, --format <FMT>` | `png` | `png` or `jpeg` |
| `--dpi <N>` | `150` | Rasterization DPI |
| `-q, --quality <1-100>` | `85` | JPEG quality (ignored for PNG) |
| `--max-edge <N>` | unset | Downscale so the long edge does not exceed N pixels (Lanczos3) |
| `--flat` | off | Flat layout: all pages written directly into `<out>/` |
| `-j, --concurrency <N>` | `1` | Document-level parallelism (rayon thread pool) |
| `--strict` | off | Stop on first failure (default: continue-on-error) |
| `--gotenberg-url <URL>` | `http://localhost:3000` | Gotenberg base URL (also `GOTENBERG_URL`) |
| `--gotenberg-timeout <SEC>` | `120` | Gotenberg request timeout |

Output layout (default, no `--flat`): each input gets its own subdirectory `<out>/<stem>/page-NNN.<ext>`. Colliding stems are disambiguated automatically (`<stem>-2/`, `<stem>-3/`, …).

Examples:

```sh
pageseer report.pdf --dpi 200
pageseer a.pdf b.pdf c.pdf -o ./pages
pageseer report.docx --format jpeg --quality 80 -o ./out
pageseer deck.pptx --max-edge 2048
pageseer a.pdf b.pdf --strict -j 4
pageseer doc.docx --gotenberg-url http://gotenberg.internal:3000
```

Exit codes: `0` success, `1` all documents failed, `2` partial failure, `64` invalid arguments or configuration error.

On any failure, `<output>/errors.json` is written with per-document per-page details (1-based page numbers, stage IDs `source-read`, `convert`, `rasterize`, `write`).

## Library

```rust
use pageseer::{extract, ImageFormat, Options, SourceInput};

let inputs = vec![
    SourceInput::Path("report.pdf".into()),
    SourceInput::Path("deck.pptx".into()),
];
let report = extract(
    &inputs,
    Options { format: ImageFormat::Png, dpi: 200, ..Options::default() },
)?;

println!(
    "{}/{} pages OK across {} documents",
    report.summary.pages_succeeded,
    report.summary.pages_succeeded + report.summary.pages_failed,
    report.summary.documents_total,
);
```

`extract` is synchronous. Per-document results are in `report.documents`; aggregate counts are in `report.summary`. Init-time errors (empty input, flat-mode stem collision, output dir creation failure) are returned as `Err`. All per-document failures (unsupported format, conversion error, rasterization failure) appear as `DocumentOutcome::Failed` inside `report.documents` rather than propagating as `Err`.

Use `extract_with_progress` to receive per-document events on a `ProgressSink` implementation.

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
