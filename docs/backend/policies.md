# Risk / Safety Policies

This project is a CLI-based ACP agent adapter. It can execute commands, apply patches, and handle
approvals, so it needs clear safety expectations.

## 1. Logging Policy

We intentionally separate:

- backend-native logs (for example `CODEX_HOME` for Codex)
- canonical logs (`ACP_HOME`, default `~/.acp`)

Canonical logs are best-effort and should never break agent execution if disk I/O fails.

## 2. Secrets Policy (Redaction + Expectations)

Canonical logging currently applies **minimal redaction** (for example `sk-...` style tokens).
This is not a complete secret detection system.

Practical guidance:

- Do not paste long-lived secrets into chat.
- Prefer environment variables / secret managers.
- Treat `~/.acp` as sensitive and keep file permissions tight.

## 3. Embedded Context Policy

Embedded text resources (for example file content attached by the client) may contain secrets.

Default behavior:

- Canonical logs do not duplicate embedded text contents.
- Only metadata is logged: URI + length.

Opt-in:

- Set `ACP_LOG_EMBEDDED_CONTEXT=1` if you want embedded text contents stored in the canonical log.

## 4. Approvals Policy

Approvals are first-class:

- When a backend requires permission (exec/apply_patch/etc.), the adapter forwards that to the ACP
  client via `RequestPermission`.
- Canonical logs record both the request and the outcome.

This makes it easier to audit what happened and why.

## 5. Global Store: When You Want It vs When You Don't

Global canonical storage is useful when you want:

- continuity across clients (editor/CLI) and future backends
- a single “timeline” of actions (plan, tool calls, approvals)

But you might disable/avoid it if:

- you have strict storage/compliance constraints
- you don't want any duplicate logs outside the backend's native storage

You can point it at a controlled location with `ACP_HOME`.

## 6. Default Execution Protocol Policy (All Use Cases)

When using `xsfire-camp`, the default operating protocol is rubric-driven iteration:

- Lock a one-sentence `Goal` with verifiable completion criteria.
- Define a `Rubric` split into `Must` and `Should`.
- Every `Must` item must include evidence (file path, command result, or primary source).
- Execute in sequence:
  `Research -> Plan -> Implement -> Verify -> Score`.
- Keep iterating until `Must` reaches 100%.
- Keep Plan UI updated every iteration until the rubric is fully satisfied.

Practical default in this repository:

- Start with `/setup` so the Plan panel exposes protocol and verification progress.
- Use `/status`, `/monitor`, and `/vector` to keep execution/verification state visible.
