# Installation & Settings Sharing

이 ACP 바이너리를 설치했을 때 기존 Codex CLI에 설정된 환경(로그인, 슬래시 커맨드, 메타데이터 등)을 그대로 가져오려면 다음을 지켜주세요.

## 1. `CODEX_HOME` 통일

Codex CLI와 ACP가 같은 설정/세션을 공유하려면 `CODEX_HOME` 환경 변수를 반드시 동일하게 설정합니다. 기본값은 `~/.codex`이며, 구버전 환경에서는 `~/.config/codex`일 수 있습니다.

```bash
CODEX_HOME="$HOME/.codex" xsfire-camp
```

Zed 등의 ACP 클라이언트에 에이전트를 등록할 때도 `command` 필드에 위와 같이 `CODEX_HOME=` 접두를 붙이거나, `settings.json`의 `agent_servers.xsfire-camp.env`에 `"CODEX_HOME": "/Users/you/.codex"`처럼 **절대 경로**를 넣으세요. (Zed는 `env`의 `$HOME`를 확장하지 않는 경우가 있습니다.) 그러면 CLI에서 만든 `settings.toml`, `threads/`, `rollouts/`, `credentials/`가 그대로 재사용됩니다. (구버전 CLI 홈을 쓰는 경우 값만 `~/.config/codex`로 맞추면 됩니다.)

## 2. 로그인 세션/자격증명 재사용

`CODEX_HOME` 아래 `credentials/`와 `login/` 디렉토리에 저장된 ChatGPT/OpenAI API 토큰을 그대로 읽기 때문에, ACP를 처음 실행할 때 별도 로그인 없이 CLI 상태를 사용할 수 있습니다. CLI에서 `codex login`한 적이 있다면 추가 작업 없이 ACP가 해당 세션을 로드합니다.

## 3. 스크립트 기반 설정 유지

- `scripts/zed_settings_backup.sh`와 `scripts/zed_settings_restore.sh`을 활용해 Zed 설정(`settings.json`)을 백업/복원하면 `agent_servers` 정의와 env 설정도 함께 보관됩니다.
- 설치/업데이트 자동화가 필요하면 `scripts/build_and_install.sh`를 `CODEX_HOME`과 함께 실행하면 됩니다.

## 4. 설치 팁 요약

| 항목 | 설명 |
| - | - |
| 바이너리 설치 | `scripts/build_and_install.sh`로 `xsfire-camp`를 `$HOME/.local/bin` 등에 설치하세요. |
| 환경 변수 | `CODEX_HOME`과 `PATH`에 설치 위치를 명시해야 CLI와 ACP가 동일한 홈을 사용합니다. |
| Zed 등록 | `settings.json`에서 `agent_servers.xsfire-camp.command`에 `CODEX_HOME`을 붙여주고, 필요하면 `env`로 추가하세요. |
| 인증 | CLI에서 사용하던 `credentials/`가 그대로 사용되므로 `OPENAI_API_KEY`, `CODEX_API_KEY` 등도 동일하게 가져갑니다. |

## 참고 자료
- `docs/event_handling.md`: ACL 이벤트 ↔ ACP 알림 흐름을 확인할 때 참고.
- `npm/README.md`: Zed 외부 에이전트 등록 가이드 요약.
