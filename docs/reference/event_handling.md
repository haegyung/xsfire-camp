# Event Handling Coverage

다음 표는 `PromptState` / `TaskState`가 처리하면서 ACP 클라이언트로 전달하는 주요 `EventMsg`와 대응하는 ACP 출력(주로 `SessionUpdate` 또는 permission 요청)을 정리한 것입니다. Codex CLI의 beta 기능이 ACP를 통해 gate 없이 그대로 전달되도록 흐름을 문서화했습니다.

| EventMsg | ACP 출력 | 설명 |
|---|---|---|
| `PlanUpdate` | `SessionUpdate::Plan` | 계획 업데이트를 `Plan` notification으로 전달해, 클라이언트가 단계별 진행 상황을 실시간으로 볼 수 있게 합니다. |
| `ExecApprovalRequest` | `RequestPermission` (비동기 approve/abort 옵션) | 명령 실행 전 승인이 필요할 때, `ToolCallUpdate`와 함께 옵션을 보여주고 결정 결과를 Codex에 전달합니다. |
| `ExecCommandBegin` / `ExecCommandOutputDelta` / `ExecCommandEnd` | `ToolCall`, `ToolCallUpdate` | 명령 시작 시 ToolCall 생성, 출력/terminal stream, 종료(성공/실패) 상태를 잇따라 업데이트합니다. |
| `TerminalInteraction` | `ToolCallUpdate` (meta terminal_output) | 터미널 stdin/출력을 메타로 내려보내 클라이언트 터미널 뷰를 활성화합니다. |
| `McpToolCallBegin` / `McpToolCallEnd` | `ToolCall`, `ToolCallUpdate` | MCP 도구 호출의 시작/완료를 ToolCall 형태로 전달하고 상태/결과를 이어서 표시합니다. |
| `ApplyPatchApprovalRequest`, `PatchApplyBegin`, `PatchApplyEnd` | `ToolCall`/`Plan` 흐름 | apply_patch 관련 승인/패치 상태도 ToolCall/Plan으로 표현합니다. |
| `RequestUserInput`, `DynamicToolCallRequest` | `RequestPermission` | 추가 입력이나 dynamic tool 요청은 `RequestPermission`으로 옵션을 제공하고 선택을 Codex에 피드백합니다. |
| `ReviewMode`(`Entered`/`Exited`, `ReviewOutput`) | `SessionUpdate::Review` 순환 (간접) | 리뷰 진입/종료 이벤트는 로그로 남고 `review_mode_exit`에서 `SessionUpdate`를 통해 검토 결과를 노출합니다. |
| `WebSearchBegin` / `WebSearchEnd` | `ToolCall`, `ToolCallUpdate` | 웹 검색을 ToolCall로 노출한 뒤 쿼리/상태를 업데이트합니다. |
| `StreamError`, `Error`, `TurnAborted` | `StopReason::Cancelled` | 오류나 중단 시 클라이언트에게 `StopReason::Cancelled`를 보내고 제출이 종료되었음을 알립니다. |
| `AgentMessage` 및 delta variants | `SessionUpdate::AgentText`/`send_agent_thought` | 모델의 텍스트/추론 출력을 스트리밍하며 text/thought로 재전송합니다. |

`TaskState`는 `PromptState`를 감싸므로 위 이벤트 처리 로직을 그대로 공유합니다. 따라서 `/compact`, `/undo` 등 Task 전용 명령도 beta 기능 관련 이벤트와 매핑된 ACP output을 그대로 받습니다.

## Setup wizard / task monitoring 보강 동작

- `/setup`를 한 번 실행하면 setup wizard plan이 활성화됩니다.
- setup wizard plan은 기본 실행 프로토콜(`Goal -> Rubric -> Research -> Plan -> Implement -> Verify -> Score`)과
  `Must=100%` 반복 게이트를 포함해, 전 usecase 기본 원칙을 Plan UI에 노출합니다.
- 활성화 이후에는 다음 동작이 있을 때 setup plan이 즉시 재발행되어 Progress가 갱신됩니다.
  - config 변경: `SetMode`, `SetModel`, `SetSessionConfigOption`
  - 검증 명령 실행: `/status`, `/monitor`, `/vector`
- setup 검증 단계는 `Verify: run /status, /monitor, and /vector`이며, 실행 상태에 따라 `pending -> in_progress -> completed`로 변합니다.

## Task monitoring 기본값

- 기본 task 모니터링 설정:
  - `Task Orchestration`: `parallel`
  - `Task Monitoring`: `auto` (또는 `on`, `off`)
  - `Progress Vector Checks`: `on`
- `/monitor`는 plan progress 외에 task queue snapshot(활성 task/submission 목록)을 출력합니다.
- `/monitor retro`는 회고형 상태 보고서(레인별 진행률/리스크/다음 작업)를 텍스트로 출력합니다.
- `Task Orchestration`이 `sequential`일 때 활성 task가 있으면 새 task 제출 대신 안내 메시지를 반환합니다.
