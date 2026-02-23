# README 개선안 (v0.9.15 기준)

## Goal
`README.md`를 "처음 보는 사용자도 2~3분 안에 설치/실행/검증 경로를 이해"할 수 있는 구조로 재정리한다.

완료 조건:
1. 상단 120줄 내에 `무엇인지`, `바로 실행`, `필수 환경`, `주요 명령`, `문서 인덱스`가 모두 노출된다.
2. KR/EN 중복 서술은 "요약 + 링크" 구조로 줄이고, 상세 설명은 `docs/`로 이동한다.
3. 유지보수자가 릴리즈 시 갱신해야 할 항목(버전/링크/검증 명령)이 체크리스트로 분리된다.

## Research
근거:
1. 현재 README는 한국어/영어 장문이 한 파일에 혼합되어 길이가 길고(상세 문단 다수), 탐색 비용이 높다.
2. 핵심 사용 경로(빌드/실행/검증)는 이미 상단에 있으나, 이후 섹션 밀도가 높아 신규 사용자가 어디까지 읽어야 하는지 판단이 어렵다.
3. 상세 정책/백엔드/품질 문서는 이미 `docs/`에 존재하므로 README는 "인덱스형 진입점"으로 재설계하는 편이 유지보수에 유리하다.

## Rubric
### Must
1. 상단 "Quick Start"를 단일 경로로 정규화한다.
Evidence: `README.md` 첫 섹션에 `Build -> Run -> Verify -> Next Docs` 순서가 존재해야 함.
2. 중복 설명을 제거하고 상세를 `docs/`로 이관한다.
Evidence: README 본문에서 정책/로드맵/백엔드 깊은 설명이 요약 1~2문단 + 링크로 대체되어야 함.
3. 버전별 릴리즈 업데이트 포인트를 고정한다.
Evidence: `README 업데이트 체크리스트` 섹션에 최소 5개 항목 포함.

### Should
1. 한국어/영어를 완전 이중 유지하지 않고 "Primary + Mirror Summary" 방식으로 압축한다.
2. "누구를 위한 도구인지"를 상단에서 명확히 분리한다(개인 개발자/팀 운영자/ACP 클라이언트 사용자).

## 적용 설계 (순서의존)
1. 정보 구조 재배치:
- Hero (1문장 가치 제안)
- Quick Start (4단계)
- Prerequisites / Environment
- Common Commands
- Integration (Zed/VSCode)
- Docs Index
- Release Update Checklist

2. 장문 섹션 정리:
- "방향성/철학/로드맵"은 3~5줄 요약만 남기고 상세는 `docs/plans/roadmap.md`로 링크.
- "기술 메모" 상세 항목은 `docs/reference/*` 링크 중심으로 축약.

3. 운영 섹션 추가:
- "Troubleshooting (Top 5)" 추가:
  - 인증 키 미설정
  - `CODEX_HOME` 불일치
  - 백엔드 CLI 미설치
  - npm 패키지 미반영
  - Zed PR/registry 반영 대기

## README 업데이트 체크리스트 (릴리즈용)
1. 버전 문자열(예: `v0.9.15`) 최신화
2. 실행 예시 명령이 현재 플래그와 일치하는지 확인
3. 검증 명령(`cargo test`, npm 테스트) 유효성 확인
4. 문서 링크(`docs/`) 존재/경로 확인
5. npm/zed registry 상태 문구 최신화
6. 릴리즈 노트 문서 링크 추가 여부 확인

## 권장 목차 스케치
1. xsfire-camp
2. Why xsfire-camp
3. Quick Start
4. Prerequisites
5. Run Modes (`--backend=...`)
6. Client Integration (Zed/VSCode)
7. Commands Snapshot
8. Troubleshooting
9. Docs Index
10. Release Checklist
11. English Summary

## 실행 계획
1. 1차: README에서 중복/장문을 `docs/` 링크 중심으로 축약
2. 2차: Quick Start와 Troubleshooting 보강
3. 3차: KR/EN 균형 조정 및 릴리즈 체크리스트 고정

## 현재 상태
이 문서는 "개선안 작성" 완료본이며, 아직 `README.md` 본문을 직접 수정하지 않았다.
다음 이터레이션에서는 위 목차 스케치 기준으로 실제 README를 패치한다.
