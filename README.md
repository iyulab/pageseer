# pageseer

> 문서를 페이지 이미지로 — VLM 파이프라인을 위한 문서 래스터라이저

**상태:** v0.1 설계 단계 (구현 미착수). 본 README는 타겟 사양을 기술한다.

`pageseer`는 PDF·Office(DOCX/XLSX/PPTX/…)·HWP 문서를 입력받아 **페이지 단위 이미지**(PNG/JPEG)로 변환해 파일 시스템에 저장하는 Rust CLI 및 라이브러리이다. 주 용도는 **VLM(비전 언어 모델) 입력 파이프라인의 전처리 단계**다.

## 이 프로젝트가 해결하는 문제

VLM 기반 문서 이해·OCR 파이프라인은 보통 "문서 → 페이지별 이미지"라는 공통 전처리 단계를 요구한다. 그러나 입력 포맷이 다양(PDF, Office, HWP)할수록 이 단계는 파편화된다 — 포맷마다 렌더링 라이브러리가 다르고, 배포·의존성 관리가 복잡해진다.

`pageseer`는 이 단계를 **하나의 파이프라인**으로 통일한다:

```
PDF ─────────────────────────────────────┐
Office → Gotenberg (LibreOffice) ─┐      │
HWP → rhwp (HWP→SVG→PDF) ─────────┤      │
                                  ▼      ▼
                                   PDF ──▶ pdfium-render ──▶ PNG/JPEG 페이지
```

모든 입력을 PDF로 정규화한 뒤 단일 라스터라이저(`pdfium-render`)로 흘린다. 외부 서비스 의존은 **Office 포맷 한정 Gotenberg 원격 HTTP 서버 1개**뿐이다.

## 주요 기능 (v0.1)

- **입력 포맷:** PDF, DOCX/DOC, XLSX/XLS, PPTX/PPT, HWP/HWPX
- **출력:** 페이지별 PNG 또는 JPEG
- **해상도 제어:** DPI 기반 + 긴 변 최대 픽셀 제한 옵션
- **배치 처리:** 여러 입력 파일 동시 처리, 부분 실패 계속(continue-on-error) 기본
- **진단:** 실패 페이지 구조화 보고 (`errors.json`)
- **플랫폼:** Linux x86_64 / Windows x86_64 / macOS Apple Silicon
- **CLI + Rust 라이브러리** 양쪽 제공

## 비전 (후속 버전)

| 버전 | 범위 |
|------|------|
| v0.1 | Rust core + CLI |
| v0.2 | C ABI + C#/Python 바인딩 |
| v0.3 | VLM 어댑터 (OpenAI·Anthropic·로컬 llama.cpp) — 페이지 이미지 → OCR/설명 |
| v0.4 | pdfium 정적 링크, 단일 바이너리 배포 |
| v0.5+ | 스트리밍 API, JSON 로그, Gotenberg 인증 |

자세한 내용은 [ROADMAP.md](./ROADMAP.md) 참조.

## 사전 요구사항

- **Rust** 1.75+
- **Gotenberg** 서버 (Office 포맷을 처리할 때만 필요)
  - Docker: `docker run --rm -p 3000:3000 gotenberg/gotenberg:8`
  - PDF만 처리한다면 Gotenberg 불필요
- **PDFium** shared library — 플랫폼별 prebuilt 바이너리 제공 예정 (v0.1 동적 로딩)
- **한글 폰트** (HWP 처리 시) — 맑은 고딕, 바탕 등 시스템 폰트 권장

## 빠른 시작 (S1: PDF 전용 — 구현 진행 중)

S1 슬라이스는 PDF 입력만 지원한다. Office/HWP는 후속 슬라이스(S3·S4) 진행 중.

**사전 준비:**

1. PDFium shared library 다운로드: <https://github.com/bblanchon/pdfium-binaries/releases/tag/chromium/7802>
   - 플랫폼 파일을 `<repo>/pdfium/`에 배치 (Windows: `pdfium.dll`, Linux: `libpdfium.so`, macOS: `libpdfium.dylib`)
   - 또는 시스템 라이브러리 검색 경로에 배치
2. Rust 1.75+ (`rustup show` 확인)

**실행:**

```sh
cargo build --release
./target/release/pageseer input.pdf -o ./out --dpi 150
```

**산출물:** `./out/<stem>/page-001.png`, `page-002.png`, ...

**S1 옵션:** `-o/--output`(출력 디렉터리), `-f/--format png`(현재 png만), `--dpi N`(기본 150).

**S2 옵션:** `--strict` (첫 실패 시 즉시 중단; 기본은 continue-on-error).

**종료 코드:** 0(성공) / 1(전체 실패) / 2(부분 실패; `errors.json` 참조) / 64(인자·포맷 오류).

**실패 보고:** continue-on-error 모드에서 페이지 1건이라도 실패하면 `<output_dir>/<stem>/errors.json` 생성 (1-based 페이지 번호, 단계 식별자 `source-read|convert|rasterize|write`).

## 사용법 (목표 CLI — 후속 슬라이스에서 활성)

```sh
# 단일 PDF → ./out/report/page-001.png ...
pageseer report.pdf --dpi 200

# 여러 입력 + JPEG 출력
pageseer *.pdf *.docx --format jpeg --quality 85 -o ./images

# VLM 입력용: 긴 변 2048px 제한
pageseer deck.pptx --max-edge 2048 --format png

# 평면 배치 (glob 친화)
pageseer report.pdf --flat -o ./out
# → ./out/report-001.png, report-002.png, ...

# 엄격 모드 (첫 실패 시 즉시 중단)
pageseer batch/*.pdf --strict

# 원격 Gotenberg 지정
pageseer doc.docx --gotenberg-url http://gotenberg.internal:3000
```

환경 변수: `GOTENBERG_URL`, `GOTENBERG_TIMEOUT`, `PAGESEER_CONCURRENCY`, `PAGESEER_OUT`.

자세한 플래그는 설계 명세(`claudedocs/specs/`)를 참조 (로컬 전용 문서).

## 아키텍처 주의사항

- `pdfium-render`의 `thread_safe` feature는 기본 활성화되어 **모든 PDFium 호출을 내부 전역 Mutex로 직렬화**한다. 즉 `--concurrency N` 설정 시 진짜 병렬 이득은 **Gotenberg HTTP 대기 및 rhwp CPU 변환 단계**에서만 발생한다. 순수 PDF 배치에서는 `-j 1`이 합리적이다.
- HWP 경로는 [edwardkim/rhwp](https://github.com/edwardkim/rhwp)를 git dependency로 사용한다. rhwp가 crates.io 배포되기 전까지 pageseer도 crates.io 배포는 하지 않는다 — GitHub release 바이너리만 제공.

## 기술 스택

| 구성 요소 | 선택 | 이유 |
|---|---|---|
| 언어 | Rust | 성능 · 메모리 안전 · 향후 C ABI 노출 용이 |
| PDF 라스터 | [pdfium-render](https://github.com/ajrcarey/pdfium-render) | Google의 PDFium 엔진, 성숙한 Rust 바인딩 |
| Office → PDF | [Gotenberg](https://gotenberg.dev) | LibreOffice 기반, 컨테이너 1개로 해결 |
| HWP → PDF | [rhwp](https://github.com/edwardkim/rhwp) | Rust 네이티브 HWP 뷰어/변환기 |
| 이미지 인코딩 | [image](https://crates.io/crates/image) | PNG/JPEG 표준 |
| HTTP 클라이언트 | [reqwest](https://crates.io/crates/reqwest) (blocking) | 성숙한 rustls 지원 |
| CLI 파서 | [clap](https://crates.io/crates/clap) v4 | derive 매크로 |

## 빌드 & 실행 (구현 착수 후)

```sh
# 빌드
cargo build --release

# 실행
./target/release/pageseer report.pdf
```

## 기여

이 프로젝트는 설계 단계이다. 본격 기여 안내는 v0.1 구현 완료 후 추가 예정. 이슈·아이디어 환영.

## 라이선스

[MIT](./LICENSE)
