# Changelog

All notable changes to pageseer are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (0.x is permitted to break compatibility between minor versions).

## [Unreleased]

## [0.1.0] - 2026-04-27

First public release. Document-to-page-image rasterizer covering PDF, Office, and HWP inputs through a single Rust pipeline.

### Added

- Rust library API: `extract(SourceInput, Options) -> Result<ExtractReport, PageseerError>`.
- CLI binary `pageseer` with options:
  - `-o/--output <DIR>` (default `./out`), `-f/--format png|jpeg`, `--dpi N` (default 150).
  - `-q/--quality 1-100` (JPEG, default 85, real `JpegEncoder::new_with_quality` encoding).
  - `--max-edge N` Lanczos3 downscale on the long edge.
  - `--flat` flat layout `<out>/<stem>-NNN.<ext>`.
  - `-j/--concurrency N` (CLI-only in v0.1; takes effect once v0.2 introduces multi-input batching).
  - `--strict` fail-fast on first error.
  - `--gotenberg-url` / `--gotenberg-timeout` / `GOTENBERG_URL` env.
- Input formats: PDF, DOCX/DOC, XLSX/XLS, PPTX/PPT, ODT/ODS/ODP, RTF, HWP/HWPX.
- Pipeline: PDF -> pdfium-render. Office -> Gotenberg (LibreOffice) -> PDF -> pdfium-render. HWP -> rhwp (HWP -> SVG -> PDF) -> pdfium-render.
- Continue-on-error mode with structured `errors.json` per input (1-based page numbers, stage IDs `source-read|convert|rasterize|write`).
- Exit codes: `0` success, `1` total failure, `2` partial failure, `64` argument/format error.
- Platform binaries: Linux x86_64, Windows x86_64, macOS Apple Silicon (`tar.gz` archives bundling `pdfium` shared library + `README` / `ROADMAP` / `LICENSE`).
- CI matrix on three platforms with auto pdfium download, plus a dedicated Ubuntu job that runs the Office integration test against a Gotenberg service container.

### Notes

- Library is `publish = false` on crates.io until rhwp publishes a stable version.
- `Cargo.lock` is committed: this repo distributes a CLI binary, so reproducible builds (`--locked`) require the lock file in VCS.
- HWP support requires Korean fonts on the host; missing fonts degrade visual quality but do not fail the pipeline.
- `--concurrency N` is exposed in v0.1 but has no effect until v0.2 adds multi-input batching with rayon (see `ROADMAP.md`).

[Unreleased]: https://github.com/iyulab/pageseer/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/iyulab/pageseer/releases/tag/v0.1.0
