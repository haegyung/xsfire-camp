# thePrometheus Codex ACP

에디터가 바뀌어도 작업을 이어가고, AI가 실행한 명령과 수정 내역을 승인까지 포함해 깔끔하게 남깁니다.

ACP 표준으로 Codex CLI를 IDE에 연결해, 작업 세션을 Tool call/Plan/승인 로그로 구조화해 보여줍니다.

Use [Codex](https://github.com/openai/codex) from [ACP-compatible](https://agentclientprotocol.com) clients such as [Zed](https://zed.dev)!

This fork aligns ACP session metadata with Codex CLI, so Zed ACP threads share the same
session source as CLI sessions while preserving ACP behavior.

Learn more about the [Agent Client Protocol](https://agentclientprotocol.com/).

## 총정리 (KR)

`theprometheus-codex-acp`는 **Codex CLI(codex-rs)** 를 **ACP(Agent Client Protocol)** 에이전트로 감싸, Zed/VS Code(ACP 확장) 같은 ACP 클라이언트에서 Codex를 “대화”가 아니라 **작업 실행이 포함된 세션**으로 운용하게 해줍니다.
핵심 가치는 **CLI 세션과 ACP 세션이 동일한 `CODEX_HOME` 저장소/메타데이터를 공유**하도록 맞춰, 클라이언트가 달라도 작업 흐름이 끊기지 않는다는 점입니다.

### 가지고 있는 기능

- ACP 표준 I/O(stdio)로 동작하는 Codex 에이전트
- 세션 저장소 공유: ACP `session_id`를 Codex thread id와 동일하게 사용하고, 세션 소스를 CLI와 맞춰 동일한 세션 저장소(`CODEX_HOME`)를 사용
- (옵션) 글로벌 세션 저장소: backend-native 로그는 분리한 채, `ACP_HOME`(기본 `~/.acp`)에 canonical 작업 로그(JSONL)를 추가로 남겨 “모델/클라이언트가 바뀌어도” 맥락을 이어가기 쉽게 함 (`docs/session_store.md`)
- Embedded context / @-mentions, 이미지 입력 지원(클라이언트가 제공할 때)
- Tool calls(쉘 실행, apply_patch, 웹 검색, MCP tool call 등) 스트리밍 및 결과 업데이트
- 승인(Approvals) 플로우: 실행/패치 등 위험 동작을 `RequestPermission`으로 노출하고 사용자 선택을 반영
- Plan/TODO/Terminal 등 “작업 진행” 신호를 ACP `SessionUpdate`로 전달
- Codex CLI parity 중심의 slash commands 지원: `/review`, `/compact`, `/undo`, `/init`, `/sessions`, `/load`, `/mcp`, `/skills` 등
- Custom prompts: 저장된 prompt를 `/name KEY=value` 형태로 호출, `$1..$9`, `$ARGUMENTS` 및 named placeholder 지원
- MCP 서버 병합: ACP 클라이언트가 제공한 MCP 서버(HTTP/stdio)를 codex-rs 설정에 병합

### 효과 (왜 유용한가)

- **클라이언트 독립성**: Zed 등 ACP 클라이언트가 바뀌어도 에이전트(이 바이너리)를 고정하면 워크플로가 안정적입니다.
- **세션 연속성**: IDE(ACP)에서 시작한 작업을 CLI에서 이어가거나, 반대로도 가능합니다(같은 `CODEX_HOME`을 쓸 때).
- **(지향) 모델/백엔드 연속성**: backend별 고유 기능은 유지하면서도, canonical 로그로 “작업 타임라인”을 통일해 LLM이 바뀌어도 맥락을 이어가기 쉬운 구조를 목표로 합니다.
- **추적 가능성**: Tool call/Plan/Terminal 같은 “행동”이 구조화되어 남아, 무엇을 했는지 검토/공유가 쉽습니다.
- **안전한 자동화**: 승인 단계를 통해 파괴적 명령이나 패치 적용을 통제하기 좋습니다.
- **재사용 가능한 협업 자산화**: Custom prompts를 템플릿화해 개발/창작 루틴(리뷰, 문서화, 교정, 요약 등)을 반복 실행 가능한 “도구”로 만들 수 있습니다.

### 방향성 (지향점)

- Codex CLI의 주요 워크플로를 ACP에서 **동등한 경험(parity)** 으로 제공
- ACP 클라이언트별 차이는 “어댑터 내부에서 흡수”하고, 사용자 입장에서는 동일한 세션/권한/툴콜 모델로 사용
- 안전성 우선: sandbox/approval 모델을 명확히 하고, 변경 가능한 영역(세션 루트 등)을 좁게 유지
- 문서와 테스트를 통해 “작동 방식”이 재현 가능하게 유지(특히 slash command/tool call/approval 스트리밍)

### 로드맵 (요약)

- 지금: Codex CLI를 ACP 에이전트로 연결하고 `CODEX_HOME`을 공유해 IDE/CLI 간 세션을 이어갑니다.
- 다음: backend driver(드라이버) 구조로 분리해 Claude Code/Gemini CLI 같은 **CLI 기반 백엔드**를 추가하고, 각 백엔드의 툴콜/승인/파일수정 “고유 기능”을 최대한 보존합니다.
- 나중: canonical 로그 스키마/상관관계 ID/보안(레닥션) 정책을 강화해 “모델이 바뀌어도” 작업 맥락을 더 안정적으로 이어가게 합니다.

자세한 계획: `docs/roadmap.md`, `docs/backends.md`, `docs/session_store.md`, `docs/policies.md`.

### 이용 케이스

개발 업무:

- 코드 변경 후 `/review`로 이슈 탐지 및 개선 루프 반복
- `/review-branch <branch>` 또는 `/review-commit <sha>`로 비교 기반 리뷰
- `/diff`로 변경사항 확인(환경에 따라 tool call로 스트리밍)
- 긴 대화/작업 후 `/compact`로 컨텍스트 압축, 필요시 `/undo`로 최근 턴 되돌리기
- MCP 도구(사내 API, 문서 검색, 티켓 시스템 등)를 붙여 “리서치/실행/정리”를 한 세션에서 처리

창작/기획/문서 작업:

- Custom prompt로 “톤/형식/검수” 템플릿을 표준화: 예) `/rewrite STYLE=formal AUDIENCE=devs`
- Plan/TODO/툴콜 결과를 기반으로 초안 → 편집 → 검수 과정을 단계화
- MCP로 외부 자료 정리/요약 파이프라인을 연결해 반복 작업을 자동화

팀 운영:

- 승인 프리셋(Approval Preset) 기반으로 “무엇을 자동으로 허용할지” 팀 기준을 맞춤
- `CODEX_HOME`을 통일해 세션/설정/인증 상태를 팀 내 운영 가이드로 고정

## 사용 방법 (KR)

### 요구 사항

- Rust toolchain(빌드 시)
- ACP 클라이언트(예: Zed) 또는 ACP를 실행할 수 있는 클라이언트
- 인증: `OPENAI_API_KEY` 또는 `CODEX_API_KEY` 또는 ChatGPT subscription(환경에 따라)
- 동일한 사용자 계정에서 `CODEX_HOME`을 공유하는 것을 권장
- (옵션) 글로벌 canonical 로그: `ACP_HOME` (기본 `~/.acp`) 및 정책은 `docs/session_store.md`, `docs/policies.md` 참고

### 설치/실행 (바이너리)

빌드:

```
cargo build --release
```

바이너리 경로:

```
target/release/theprometheus-codex-acp
```

실행(ACP stdio 에이전트로 동작):

```
OPENAI_API_KEY=sk-... CODEX_HOME="$HOME/.codex" target/release/theprometheus-codex-acp
```

### Zed (custom agent registration)

Zed에 custom ACP agent로 등록하면, Zed 내장 Codex 어댑터 변화와 무관하게 설정을 고정할 수 있습니다.

`settings.json` 예시(경로는 환경에 맞게):

```
{
  "agent_servers": {
    "thePrometheus Codex ACP": {
      "type": "custom",
      "command": "/absolute/path/to/theprometheus-codex-acp",
      "env": {
        "CODEX_HOME": "/Users/you/.codex"
      }
    }
  }
}
```

Agent Panel에서 "thePrometheus Codex ACP"로 새 스레드를 시작합니다.

### VS Code

이 레포는 VS Code 확장(ACP 클라이언트)을 포함하지 않습니다.
VS Code에서 사용하려면 “ACP 클라이언트 역할”을 하는 확장/플러그인이 별도로 필요하며, 해당 확장이 stdio 기반 커스텀 에이전트를 실행할 수 있어야 합니다.

커뮤니티 “VSCode ACP” 확장을 사용하는 경우, 에이전트를 `<command> acp` 형태로 실행하는 구현이 있을 수 있습니다.
이 바이너리는 `acp`/`--acp` 인자를 받아도 동일하게 ACP 에이전트로 동작하도록 호환되어 있으므로 아래 형태로도 실행될 수 있습니다:

```
theprometheus-codex-acp acp
```

VS Code 확장이 PATH에서 에이전트를 찾는 방식이라면, 다음 중 하나로 `theprometheus-codex-acp` 커맨드를 PATH에 노출하세요.

```
npm i -g @haegyung/theprometheus-codex-acp
```

또는 직접 빌드한 바이너리를 PATH에 두고 실행해도 됩니다.

확장에서 환경변수 주입을 지원하지 않는 경우, VS Code를 환경변수와 함께 실행하는 방식이 가장 확실합니다:

```
CODEX_HOME="$HOME/.codex" OPENAI_API_KEY=sk-... code .
```

### npm으로 실행

```
npx @haegyung/theprometheus-codex-acp
```

## 기술 메모 (KR)

- ACP는 stdio로 연결됩니다. 이 바이너리는 ACP 메시지를 codex-rs의 thread/session 실행으로 브릿지하고, 결과를 `SessionUpdate`로 스트리밍합니다.
- 세션 저장소 공유를 위해 ACP `session_id`는 Codex thread id와 동일하게 취급합니다. 또한 세션 소스를 CLI로 맞춰 동일한 메타데이터 저장소(`CODEX_HOME/threads`, `CODEX_HOME/rollouts`)를 공유합니다.
- `new_session`/`load_session` 시 `cwd`(세션 루트)를 기록하고, 파일 접근은 기본적으로 이 루트 밖 경로를 차단합니다.
- 일부 CLI 커맨드는 인터랙티브 메뉴를 전제로 하므로 ACP에서 안내 메시지로 대체될 수 있습니다.
- 세션/스레드 전환은 ACP 클라이언트가 주도해야 합니다(`/load`는 전환 방법을 안내).

참고:

- `CODEX_HOME` 구조/권한: `docs/codex_home_overview.md`
- 이벤트 -> ACP 출력 매핑: `docs/event_handling.md`
- 로컬 검증 가이드: `docs/verification_guidance.md`

## How to use (EN)

This repository is documented primarily in Korean above. This section is a short English quick start.

### Quick start (binary)

Build:

```
cargo build --release
```

Run (ACP agent over stdio):

```
OPENAI_API_KEY=sk-... CODEX_HOME="$HOME/.codex" target/release/theprometheus-codex-acp
```

### Quick start (npm)

Run:

```
npx @haegyung/theprometheus-codex-acp
```

Install globally (to expose `theprometheus-codex-acp` on PATH):

```
npm i -g @haegyung/theprometheus-codex-acp
```

### Clients

- Zed: register this binary as a custom ACP agent (see `사용 방법 (KR)` above for a complete `settings.json` example).
- VS Code: requires a community ACP client extension. Some extensions run agents as `<command> acp`; this binary accepts `acp`/`--acp` as no-ops for compatibility.
- Other ACP clients: see [ACP compatible clients](https://agentclientprotocol.com/overview/clients).

### Automation

```
scripts/build_and_install.sh
scripts/tag_release.sh vX.Y.Z
```

### Verification

```
cargo test
node npm/testing/test-platform-detection.js
```

### Releases

If you need a prebuilt binary, see:
[haegyung/theP_codex releases](https://github.com/haegyung/theP_codex/releases)

## License

Apache-2.0
