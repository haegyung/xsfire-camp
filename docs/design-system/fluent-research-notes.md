# Fluent Design Research Notes

기준일: 2026-02-14  
범위: Microsoft Fluent 2 Design System + Fluent UI Web Components

## 공식 문서 출처

- https://fluent2.microsoft.design/design-tokens
- https://fluent2.microsoft.design/typography
- https://fluent2.microsoft.design/layout
- https://fluent2.microsoft.design/elevation
- https://fluent2.microsoft.design/color
- https://fluent2.microsoft.design/motion
- https://fluent2.microsoft.design/iconography
- https://fluent2.microsoft.design/accessibility
- https://learn.microsoft.com/en-us/fluent-ui/web-components/getting-started/styling
- https://learn.microsoft.com/en-us/fluent-ui/web-components/design-system/high-contrast
- https://learn.microsoft.com/en-us/fluent-ui/web-components/components/button
- https://learn.microsoft.com/en-us/fluent-ui/web-components/components/anchor

## 핵심 정리 (코드 반영 우선순위)

### 토큰 정책
- Fluent는 의미 토큰 중심 설계를 권장한다.  
  - 글로벌 레벨 토큰(원시 값) + 앨리어스 토큰(의미 레이어) 분리
- 색상/타이포/간격/보더/포커스/모션 모두 토큰 기반으로 관리
- 상호작용 상태는 `rest → hover → pressed/selected`로 상태 토큰을 분리 관리

### 타입/타이포
- 텍스트 레벨 체계를 사용해 가독성 우선의 계층화
- 기본 본문, 제목, 보조 제목, 캡션 등을 구분
- `Segoe UI` 등 시스템성 글꼴 기반 체감 통일

### 레이아웃/터치
- spacing ramp를 단계화해 레이아웃 통일성 유지
- 터치 타깃 권장치 반영 필요
  - 웹/iOS: 44px
  - Android: 48px
- 일관된 컬럼/마진/거터 규칙을 기준으로 구성

### 접근성
- WCAG AA 기준을 기준선으로 사용
  - 일반 텍스트 대비율 4.5:1
  - 큰 텍스트 대비율 3.0:1
- 포커스 순서, 키보드 경로, 보조텍스트/aria 라벨, 시맨틱 역할을 명시
- 색 대비만으로 상태 전달을 하지 않도록 아이콘/텍스트/패턴 결합

### Motion 및 동작
- 모션은 목적 중심으로 제한
- “reduced motion” 경로 기본 지원
- 과도한 화면 전면 전환/깜빡임/연속 흔들림은 피함

### 고대비(Windows)
- `@media (forced-colors: active)` 및 Windows system color를 고려한 fallback 제공
- `Canvas`, `CanvasText`, `ButtonText`, `Highlight`, `GrayText` 계열 사용 시 의미 손실 최소화
- HC에서는 hover/selected/focus가 사라지지 않도록 명확한 스타일 우선순위 유지

## 구현 매핑 (요약)

| 범주 | 적용 항목 |
|---|---|
| 색상 | `--ms-color-*`, `--ms-foreground-*`, `--ms-surface-*` |
| 간격 | `--ms-spacing-*`, 높이 4/8/12/16/24 단위 기본값 |
| 타이포 | `--ms-font-family-*`, `--ms-font-size-*`, `--ms-line-height-*`, `--ms-font-weight-*` |
| 상태 | `--ms-button-bg-*`, `--ms-focus-*`, `--ms-motion-*` |
| 모드 | `data-ms-theme='light'|'dark'|'highContrast'` 분기 |
| 접근성 | `focus-visible`, `forced-colors`, `prefers-reduced-motion` 대응 |

## 사용 규칙(검증 대상)
- 하드코딩 값 제거 후 토큰 참조 강제
- 다크/라이트/고대비 토큰 값 분기
- Contrast/키보드/HC 경로를 릴리즈 QA에 강제 체크
- UI 컴포넌트는 상태별 토큰을 통해 렌더링되어야 함
