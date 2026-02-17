# 검증/테스트 안내

다음 절차로 ACP가 CLI와 동일하게 동작하는지 확인할 수 있습니다.

## 1. 자동화된 테스트

```bash
cargo test
```

현재 `thread.rs`/`prompt_args.rs`에 있는 단위 테스트가 실행되며, Task/Prompt 흐름을 포함합니다. 성공 메시지가 나오면 기본 Promise loop가 깨지지 않고 `PromptState`가 정상 작동함을 의미합니다.

## 2. 수동 검증 시나리오

CIDR–like manual verification steps:
1. `CODEX_HOME`이 CLI와 동일한지 확인하고, 같은 `threads/`/`rollouts/`/`credentials/` 디렉토리 경로를 사용하세요.
2. ACP(예: `xsfire-camp`)를 실행하고 `/setup`를 먼저 실행해 setup wizard plan을 띄웁니다.
3. `/status` -> `/monitor` -> `/vector` 순서로 실행하고, Plan의 `Verify: run /status, /monitor, and /vector`가 `pending -> in_progress -> completed`로 갱신되는지 확인합니다.
4. Config Options에서 `Model`, `Reasoning Effort`, `Approval Preset`, `Task Orchestration`, `Task Monitoring`(`on/auto/off`), `Progress Vector Checks` 중 하나를 변경하고, Plan progress가 즉시 반영되는지 확인합니다.
5. `/monitor` 출력에 다음이 보이는지 확인합니다.
   - `Task monitoring: orchestration=..., monitor=..., vector_checks=...`
   - 활성 task가 있으면 `Task queue: N active` 및 항목 목록
   - `/monitor retro` 호출 시 회고형 상태 보고서(레인/리스크/학습/다음 작업) 텍스트가 출력되는지 확인할 수 있습니다.
6. `Task Orchestration`을 `sequential`로 바꾼 뒤 task가 진행 중일 때 새 요청을 보내, 즉시 대기 안내 메시지가 나오는지 확인합니다.
7. `logs/codex_chats/<agent>/<timestamp>.md`에 새 turn이 기록되는지 확인하며, 각 turn에서 `Plan`/`ToolCall`/`RequestPermission`이 나오는지 검토합니다.
8. `docs/reference/event_handling.md`에 정리한 매핑에 따라 각 `EventMsg`(PlanUpdate, ExecCommand*, McpToolCall*, RequestUserInput 등)가 ACP notification으로 나오는지 확인하고, KVS 로그(예: `tracing` 출력)를 참고하세요.
9. 웹 인터페이스(Zed)를 사용하는 경우, slash 명령을 실행하면서 Agent Panel의 `Plan`, `Tool Calls`, `Terminal` 탭이 정상적으로 업데이트되는지 보세요.

### 실행 스크립트(체크리스트 리포트 생성)

```bash
scripts/manual_verification_setup_monitor.sh
```

- 실행 결과로 `logs/manual_verification/setup_monitor_<timestamp>.md` 리포트가 생성됩니다.
- 자동 게이트를 건너뛰고 체크리스트만 생성하려면:

```bash
scripts/manual_verification_setup_monitor.sh --skip-gates
```

## 3. 로그/도구 호출 확인

- `logs/` 디렉토리의 `codex_chats` 파일을 열어 `Plan` 업데이트나 `ToolCall` 생성 시각을 확인할 수 있습니다.
- `session_notification`을 디버깅하려면 `RUST_LOG=debug`를 활성화하고, `xsfire-camp`(또는 `cargo run --release`)을 실행해주세요.

## 4. 반복 검증

CI/QA 파이프라인에서는
1. `cargo test`를 항상 실행하고
2. `docs/reference/event_handling.md`의 이벤트-출력 매핑이 누락되었는지 유지보수 체크리스트로 사용하세요.

필요하면 위 내용을 스크립트로 자동화하거나, 오류 발생 시 log snippet을 붙여 PR 검토자에게 보여주는 것도 좋습니다.
