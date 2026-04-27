# pageseer

Rasterize documents into per-page images.

`pageseer` is a Rust library and CLI that converts PDF, Office, and HWP/HWPX files into per-page PNG or JPEG images. It is intended as a preprocessing step for downstream pipelines that operate on page images (OCR, vision-language models, search indexing).

Status: v0.1.0. See [CHANGELOG.md](./CHANGELOG.md).

## Scope

In scope (v0.1):

- Inputs: PDF, DOCX/DOC, XLSX/XLS, PPTX/PPT, ODT/ODS/ODP, RTF, HWP/HWPX
- Outputs: PNG or JPEG, one file per page
- Single-input processing via CLI or library
- Continue-on-error mode with a structured `errors.json` per input
- Platforms: Linux x86_64, Windows x86_64, macOS Apple Silicon

Out of scope (v0.1):

- Multi-input batching, page-level parallelism, rayon
- Embedded image extraction, page range selection
- VLM/OCR adapters, streaming API, JSON logging
- Authenticated Gotenberg, static pdfium linking
- crates.io publication (blocked on the rhwp dependency)

See [ROADMAP.md](./ROADMAP.md) for later versions.

## Pipeline

```
PDF  ────────────────────────────────────┐
Office → Gotenberg (LibreOffice) ─┐      │
HWP    → rhwp (HWP → SVG → PDF) ──┤      │
                                  ▼      ▼
                                  PDF ──▶ pdfium-render ──▶ PNG/JPEG
```

All inputs are normalized to PDF and rasterized with `pdfium-render`. The only external service dependency is Gotenberg, and only for Office formats.

## Requirements

- Rust 1.75 or newer
- A `pdfium` shared library (loaded dynamically). Download a build matching your platform from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) (CI pins `chromium/7763`) and place it at `<repo>/pdfium/` or in the system library search path:
  - Linux: `libpdfium.so`
  - Windows: `pdfium.dll`
  - macOS: `libpdfium.dylib`
- Gotenberg, only when processing Office formats: `docker run --rm -p 3000:3000 gotenberg/gotenberg:8`
- CJK fonts on the host when processing HWP/HWPX (Noto Sans CJK on Linux; Microsoft and Apple platforms ship suitable fonts by default)

## Install

Pre-built binaries are attached to each [GitHub release](https://github.com/iyulab/pageseer/releases). Each archive bundles the matching `pdfium` shared library alongside the `pageseer` executable.

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
| `-j, --concurrency <N>` | `1` | Reserved for v0.2 multi-input batching; no effect in v0.1 |
| `--strict` | off | Stop on first failure (default is continue-on-error) |
| `--gotenberg-url <URL>` | `http://localhost:3000` | Gotenberg base URL (also `GOTENBERG_URL`) |
| `--gotenberg-timeout <SEC>` | `120` | Gotenberg request timeout |

Examples:

```sh
pageseer report.pdf --dpi 200
pageseer report.docx --format jpeg --quality 80 -o ./out
pageseer deck.pptx --max-edge 2048
pageseer slides.pdf --flat -o ./out      # ./out/slides-001.png, ...
pageseer doc.docx --gotenberg-url http://gotenberg.internal:3000
```

Exit codes: `0` success, `1` total failure, `2` partial failure (see `errors.json`), `64` invalid arguments or unsupported format.

When at least one page fails in continue-on-error mode, `<output>/<stem>/errors.json` is written with 1-based page numbers and stage identifiers (`source-read`, `convert`, `rasterize`, `write`).

## Library

```rust
use pageseer::{extract, ImageFormat, Options, SourceInput};

let report = extract(
    SourceInput::Path("report.pdf".into()),
    Options { format: ImageFormat::Png, dpi: 200, ..Options::default() },
)?;

println!("{} pages, {} failed", report.succeeded_count(), report.failed_count());
```

`extract` is synchronous and consumes the `SourceInput`. Failures are reported as `PageseerError`; partial failures are returned via `PageseerError::Partial(report)`.

HWP processing may panic inside `rhwp` on malformed input. Callers that need isolation should wrap the call in `std::panic::catch_unwind`.

## Notes

- `pdfium-render` enables its `thread_safe` feature by default, which serializes all PDFium calls behind a global mutex. Document-level concurrency only helps for stages outside PDFium (Gotenberg HTTP, rhwp CPU work). Pure-PDF batches see no speedup from `-j > 1`.
- The HWP path depends on [`rhwp`](https://github.com/edwardkim/rhwp) as a git dependency. `pageseer` will not be published to crates.io until `rhwp` is.

## Testing

Unit tests run by default:

```sh
cargo test
```

Integration tests are gated behind `#[ignore]` and require `pdfium`:

```sh
cargo test -- --include-ignored
```

The Office integration test additionally requires a running Gotenberg server, exposed via `PAGESEER_TEST_GOTENBERG_URL`. The `sample.docx` fixture is generated at runtime via the `docx-rs` dev-dependency.

The HWP integration test requires `tests/fixtures/sample.hwp` to be supplied by the user. `rhwp` is a decoder, so the fixture cannot be generated automatically. CI does not run this test.

CI builds against Linux, Windows, and macOS, downloading `pdfium` automatically. A separate Linux job runs the Office integration test against a Gotenberg service container.

## License

[MIT](./LICENSE)
