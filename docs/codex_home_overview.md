# CODEX_HOME 구조 및 권한 체크리스트

`CODEx_HOME`은 Codex CLI/ACP가 공유하는 사용자 데이터 디렉토리입니다. 기본값은 `~/.codex`이며, 구버전 환경에서는 `~/.config/codex`일 수 있습니다. 다음과 같은 하위 항목을 갖습니다.

| 항목 | 설명 | 접근 필요성 |
| - | - | - |
| `settings.toml` | Codex 설정(autocomplete, default model, project trust 등). | ACP가 CLI와 동일한 config를 읽도록 이 파일이 같아야 합니다. 읽기 권한만 필요합니다. |
| `threads/` | 생성된 세션의 rollout/metadata. | ACP가 CLI 세션 메타데이터(`SessionMetaLine`)를 매핑할 때 읽기/쓰기. 파일 소유자와 권한이 동일해야 `ThreadManager`가 접근 가능합니다. |
| `rollouts/` | 각 session의 전체 rollout history. | 이어받기/팔로우/로드 시 읽기/쓰기. gzip/serde 처리하므로 파일이 잠겨 있지 않아야 합니다. |
| `credentials/` | ChatGPT/OpenAI API 토큰, login server 메타. | 인증 흐름에서 읽고 쓰므로 `600` 수준의 권한을 유지하십시오. ACL 상 `xsfire-camp` 프로세스와 CLI 사용자 계정이 동일해야 합니다. |
| `logs/` | 내부 로그 (Codex CLI와 공용). | 선택적이며 디버깅시 `RUST_LOG` 설정에 따라 생성됩니다. 보통 `644` 권한이면 충분합니다. |
| `workspace/` | (선택) 각 세션의 가상 작업공간. | sandbox/`RolloutRecorder`가 작업 디렉토리(세션 루트)를 만들기 때문에 쓰기 권한 필요. |

## 권한 체크리스트

1. `CODEX_HOME` 디렉토리 소유자는 Codex CLI/ACP를 실행하는 사용자와 동일해야 합니다.
2. `credentials/`와 `threads/`는 특히 쓰기 권한이 필요하므로 `chmod 600/700` 수준을 유지하세요.
3. `settings.toml`이 수정되었을 때 ACP가 재시작되면 구조를 다시 읽으므로, 락이 걸리지 않도록 보장하세요.
4. 네트워크형 remote project(RDP/ssh)에서 CLI를 실행하던 경우, ACP도 같은 서버에서 접근하도록 `CODEX_HOME`을 공유하세요.

## 유지 관리 팁

- `scripts/zed_settings_backup.sh`는 Zed `settings.json`을 백업하므로 `CODEX_HOME`과 병행해서 사용하면 설정 일관성을 유지할 수 있습니다.
- `logs/`에 움직이는 `codex_chats` 파일을 정기적으로 정리해두면 `ThreadManager` loadd 도움이 됩니다.
- `target/` 같은 빌드 출력은 `CODEX_HOME`과 분리되어야 하므로, `scripts/build_and_install.sh`를 실행할 때 `CARGO_TARGET_DIR`을 커스터마이즈하세요.
