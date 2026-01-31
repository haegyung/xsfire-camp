# 검증/테스트 안내

다음 절차로 ACP가 CLI와 동일하게 동작하는지 확인할 수 있습니다.

## 1. 자동화된 테스트

```bash
cargo test
```

현재 `thread.rs`/`prompt_args.rs`에 있는 단위 테스트(16개)가 실행되며, Task/Prompt 흐름을 포함합니다. 성공 메시지가 나오면 기본 Promise loop가 깨지지 않고 `PromptState`가 정상 작동함을 의미합니다.

## 2. 수동 검증 시나리오

CIDR–like manual verification steps:
1. `CODEX_HOME`이 CLI와 동일한지 확인하고, 같은 `threads/`/`rollouts/`/`credentials/` 디렉토리 경로를 사용하세요.
2. ACP(예: `codex-acp`)를 실행하고 `/compact`, `/review`, `/undo`, `/init` 등 beta slash 명령을 순차적으로 실행합니다.
3. `logs/codex_chats/<agent>/<timestamp>.md`에 새 turn이 기록되는지 확인하며, 각 turn에서 `Plan`/`ToolCall`/`RequestPermission`이 나오는지 검토합니다.
4. `docs/event_handling.md`에 정리한 매핑에 따라 각 `EventMsg`(PlanUpdate, ExecCommand*, McpToolCall*, RequestUserInput 등)가 ACP notification으로 나오는지 확인하고, KVS 로그(예: `tracing` 출력)를 참고하세요.
5. 웹 인터페이스(Zed)를 사용하는 경우, 해당 slash 명령을 실행하면서 Agent Panel에 `Plan`, `Tool Calls`, `Terminal` 탭이 정상적으로 업데이트되는지 보세요.

## 3. 로그/도구 호출 확인

- `logs/` 디렉토리의 `codex_chats` 파일을 열어 `Plan` 업데이트나 `ToolCall` 생성 시각을 확인할 수 있습니다.
- `session_notification`을 디버깅하려면 `RUST_LOG=debug`를 활성화하고, `codex-acp`(또는 `cargo run --release`)을 실행해주세요.

## 4. 반복 검증

CI/QA 파이프라인에서는
1. `cargo test`를 항상 실행하고
2. `docs/event_handling.md`의 이벤트-출력 매핑이 누락되었는지 유지보수 체크리스트로 사용하세요.

필요하면 위 내용을 스크립트로 자동화하거나, 오류 발생 시 log snippet을 붙여 PR 검토자에게 보여주는 것도 좋습니다.
