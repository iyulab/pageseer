# pageseer 로드맵

본 문서는 **목적지**만 담는다. 세부 구현 계획은 별도 (claudedocs/plans/, 로컬 전용).

## 북극성

**문서 → 페이지 이미지 → VLM 파이프라인**의 전처리 단계를 단일 Rust 코어로 통일한다. C ABI 경유로 C#·Python 등 다언어 소비자가 동일 엔진을 공유할 수 있게 한다.

---

## v0.1 — 코어 + CLI ✅

**범위:** 문서 → 페이지 이미지 변환의 엔드투엔드 검증.

- Rust 라이브러리 공개 API: `extract(SourceInput, Options) -> Result<ExtractReport, PageseerError>`
- CLI 바이너리 `pageseer` (PDF/Office/HWP 입력, PNG/JPEG 출력)
- 파이프라인: PDF → pdfium / Office → Gotenberg → pdfium / HWP → rhwp → pdfium
- 부분 실패 계속 모드 (`--strict` 반대) + `errors.json`
- Linux x86_64 / Windows x86_64 / macOS (Apple Silicon) 바이너리 릴리스

**완료 기준:**
1. 3개 플랫폼에서 각 포맷(PDF/DOCX/XLSX/PPTX/HWP) 샘플 엔드투엔드 성공
2. CLI 통합 테스트 (Gotenberg docker-compose)
3. GitHub release 바이너리 배포 파이프라인 green

**비범위:** VLM, FFI, 정적 링크, crates.io 배포.

---

## v0.2 — 멀티입력 + FFI 표면

**목표:** 다중 입력 배치 처리를 정착시켜 API 표면을 안정화한 뒤 C#/Python에서 네이티브 호출.

- **멀티입력 배치 처리** — `extract`/`CLI`가 여러 입력을 한 번에 받아 부분 실패 계속 + 통합 `errors.json`. v0.1의 `-j/--concurrency` 플래그가 실효 동작 확보 (rayon 도입). 이 항목이 C ABI 직전에 들어가는 이유는 batch 시그니처가 ABI 안정성에 직접 영향을 주기 때문이다.
- `pageseer-ffi` 크레이트: C ABI 노출 (`extern "C"`), 안정 symbol 집합
- C header 자동 생성 (`cbindgen`)
- C# NuGet 패키지 초기 릴리스 (.NET standard target)
- Python wheel 초기 릴리스 (`maturin` 기반, `cffi` 또는 `ctypes` 바인딩)
- 각 바인딩에 최소 end-to-end 예제 (PDF 입력 → 이미지 파일 출력)

**결정 유보:** `pyo3`/`napi-rs` 등 언어별 FFI crate 사용 여부. v0.1 말기에 C ABI 경계 안정성이 확보되면 언어별 경로로 분기 가능성 재평가. 페이지 단위 병렬화는 pdfium thread_safe 내부 mutex 제약이 남아있어 v0.5+로 이연.

---

## v0.3 — VLM 연계

**목표:** 페이지 이미지를 VLM 프롬프트/응답으로 이어주는 어댑터 레이어.

- VLM 어댑터 추상 trait (`VlmBackend`)
- 1차 구현체:
  - OpenAI-compatible API (GPT-4V·Anthropic messages API 양쪽 포섭)
  - 로컬 llama.cpp 서버 (gguf 기반 vision 모델)
- 파이프라인: `extract → for_each_page(describe | ocr | custom_prompt)`
- 새 CLI 서브커맨드: `pageseer describe <input> --backend openai --prompt "..."`
- 결과 저장: 페이지 이미지 + JSONL (`<stem>/page-001.json`)

**결정 유보:** 로컬·원격 배치 정책 (rate limit / concurrency / retry), 장문 페이지 chunking 전략.

---

## v0.4 — 배포 단순화

**목표:** "다운로드 → 실행" 1단계.

- pdfium 정적 링크 (`pdfium-render/static` feature + 플랫폼별 prebuilt 내장)
- 단일 실행 파일 (사용자가 pdfium shared lib을 별도로 두지 않아도 됨)
- Windows MSI·macOS pkg·Linux tar 배포 파이프라인
- Docker 이미지 (Gotenberg + pageseer 일체형 옵션)

---

## v0.5+ — 런타임 다듬기

demand-driven. 아래는 "거론되었으나 v0.1–0.4 범위 밖" 항목 대기열:

- **스트리밍 API** — 페이지 라스터 완료 시마다 콜백 (대용량 문서 메모리 절약)
- **JSON 구조화 로그** (`--log-json`) — CI/파이프라인 친화
- **Gotenberg 인증** — 커스텀 헤더·basic auth
- **PDF 페이지 범위** (`--pages 1-10,15`) — 부분 추출
- **WebP/AVIF 출력**
- **임베디드 이미지 추출 모드** — 페이지 래스터 대안(원본 삽입 이미지만 뽑기)
- **페이지 단위 병렬화** — pdfium thread_safe 내부 mutex 개선 여부에 따라 재평가 (v0.2 멀티입력은 문서 단위 rayon이며 페이지 단위 아님)
- **crates.io 배포** — rhwp 배포 시점 동기

---

## 버저닝 정책

- 모든 0.X는 **하위 호환 보장 없음**. API·CLI 플래그 변경 가능.
- 0.X.0: 기능 추가 / 0.0.X: 버그픽스·문서.
- 1.0.0은 별도 이정표. 커뮤니티·소비자 안정화 요구가 모였을 때만.

---

## 의사결정 원칙

1. **근본 해결 > 표면 회피**. 변경 범위 최소화는 판단 기준이 아니다.
2. **Demand-driven growth**. 추측성 확장 금지. 실제 소비자 수요가 모였을 때만 API 표면 확장.
3. **단일 합류점**. 모든 입력은 공통 중간 표현(PDF)으로 수렴 후 공통 렌더러로 흘린다.
4. **외부 서비스는 옵셔널**. Gotenberg는 Office 경로에만 필요. PDF-only 사용자는 HTTP 의존 없음.
