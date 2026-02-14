# MS Fluent Token Schema (v1)

목적: 향후 Fluent 기반 프런트엔드(React, Web Components, MCP 위젯)에서 공통 토큰을 단일 소스로 사용할 수 있게 한다.

적용 범위:
- 색상(`color`), 간격(`spacing`), 반경(`radius`), 테두리(`stroke`), 타이포(`type`), 포커스(`focus`), 모션(`motion`)
- 다크/라이트/고대비 테마 오버레이 지원
- 컴포넌트별 토큰 매핑으로 `Button`, `Input`, `Card`, `Dialog`, `Anchor`의 최소 기본값 제공

## 1. 파일 구조 제안

- `docs/design-system/fluent-tokens.yaml` (또는 JSON): 원본 토큰 선언
- `docs/design-system/fluent-theme-provider.*`: 런타임에서 토큰 주입 로직
- `src/design-system/*` (프론트엔드 존재 시): 실제 스타일 적용 레이어
- `.css`: `--ms-*` CSS custom properties 생성

## 2. 최종 스키마

```json
{
  "$id": "https://example.org/schemas/ms-fluent-token.schema.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "MsFluentTokenPalette",
  "type": "object",
  "required": ["version", "theme", "tokens"],
  "properties": {
    "version": { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+$" },
    "theme": {
      "type": "string",
      "enum": ["light", "dark", "highContrast"]
    },
    "tokens": {
      "type": "object",
      "required": ["color", "spacing", "typography", "radius", "stroke", "focus", "motion"],
      "properties": {
        "color": {
          "type": "object",
          "required": ["background", "foreground", "surface", "brand", "status", "text"],
          "properties": {
            "background": { "type": "object", "required": ["default", "subtle", "canvas"] },
            "foreground": { "type": "object", "required": ["default", "muted", "disabled"] },
            "surface": { "type": "object", "required": ["card", "elevated", "overlay", "border"] },
            "brand": {
              "type": "object",
              "required": ["background", "hover", "pressed", "text", "focus", "selected"]
            },
            "status": {
              "type": "object",
              "required": ["success", "warning", "danger", "info"]
            },
            "text": {
              "type": "object",
              "required": ["default", "onBrand", "disabled", "link", "linkHover"]
            }
          }
        },
        "spacing": {
          "type": "object",
          "required": ["base4", "base8", "base12", "base16", "base24", "base32", "base48", "base56"],
          "properties": {
            "base4": { "type": "string", "pattern": "^\\d+px$" },
            "base8": { "type": "string", "pattern": "^\\d+px$" },
            "base12": { "type": "string", "pattern": "^\\d+px$" },
            "base16": { "type": "string", "pattern": "^\\d+px$" },
            "base24": { "type": "string", "pattern": "^\\d+px$" },
            "base32": { "type": "string", "pattern": "^\\d+px$" },
            "base48": { "type": "string", "pattern": "^\\d+px$" },
            "base56": { "type": "string", "pattern": "^\\d+px$" }
          }
        },
        "typography": {
          "type": "object",
          "required": ["family", "size", "lineHeight", "weight", "letterSpacing"],
          "properties": {
            "family": {
              "type": "object",
              "required": ["base", "code", "fallback"]
            },
            "size": {
              "type": "object",
              "required": ["caption", "body", "bodyStrong", "title", "subtitle", "display"]
            },
            "lineHeight": {
              "type": "object",
              "required": ["compact", "comfortable", "loose"]
            },
            "weight": {
              "type": "object",
              "required": ["regular", "semibold", "bold"]
            },
            "letterSpacing": { "type": "object" }
          }
        },
        "radius": { "type": "object", "required": ["small", "medium", "large", "full"] },
        "stroke": { "type": "object", "required": ["thin", "medium", "thick", "focus"] },
        "focus": {
          "type": "object",
          "required": ["color", "width", "offset", "style"]
        },
        "motion": {
          "type": "object",
          "required": ["duration", "easing", "respectReducedMotion"],
          "properties": {
            "duration": {
              "type": "object",
              "required": ["fast", "normal", "slow"]
            },
            "easing": { "type": "object" },
            "respectReducedMotion": { "type": "boolean" }
          }
        }
      }
    },
    "componentMappings": {
      "type": "object",
      "properties": {
        "button": { "$ref": "#/definitions/componentMap" },
        "input": { "$ref": "#/definitions/componentMap" },
        "card": { "$ref": "#/definitions/componentMap" },
        "dialog": { "$ref": "#/definitions/componentMap" }
      }
    }
  },
  "definitions": {
    "componentMap": {
      "type": "object",
      "required": ["cssVars", "states"],
      "properties": {
        "cssVars": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        },
        "states": {
          "type": "object",
          "required": ["rest", "hover", "active", "focus", "disabled", "selected"],
          "additionalProperties": { "type": "string" }
        }
      }
    }
  }
}
```

## 3. 샘플 토큰(실데이터) 예시

실제 값은 `docs/design-system/fluent-tokens.json`에 넣어 사용한다.  
모든 값은 `--ms-*` CSS 변수로 컴파일하거나 런타임 주입한다.

- 공통 원칙
  - 하드코딩 값 금지: 오직 토큰 식별자만 사용
  - 의미 토큰만 사용: `ms-fill-primary` 대신 `ms-bg-brand-rest` 형태
  - 상태 토큰 분리: `hover`, `pressed`, `disabled`를 별도 키로 분리
  - 다크/라이트/고대비 독립 관리

## 4. 적용 규칙(필수)

- 모든 컴포넌트는 tokenized CSS 변수로 스타일을 받음
  - 예: `background-color: var(--ms-button-bg-rest)`
- 모든 상호작용 상태는 토큰 기반
  - 예: `--ms-button-bg-hover`, `--ms-button-bg-pressed`
- `color` 속성은 배경 대비 검토 기준값을 참조
  - AA 기본 텍스트 4.5:1, 큰 텍스트 3.0:1 충족
- 포커스는 별도 토큰화
  - `--ms-focus-color`, `--ms-focus-width`
- 모션은 축소 모드 지원
  - `prefers-reduced-motion: reduce` 경로에서 duration을 0 또는 최소화

## 5. 릴리즈/운영 체크

1. 토큰 변경 시 최소 1개 버전 커밋(major/minor/patch 정책)
2. 변경된 토큰과 실제 렌더 샘플 간 차이를 스냅샷으로 캡처
3. 고대비 모드에서 기본 시나리오 스모크 4개 항목 통과
   - 기본/호버/선택/포커스
4. 토큰 릴리즈 노트(변경 목록, 롤백 가이드) 작성
