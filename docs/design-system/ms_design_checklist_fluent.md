# MS Fluent Design 리서치 & 코드 반영 체크리스트

기준일: 2026-02-14

## 수집 출처 및 원본 정리
- 정식 리서치 요약: `docs/design-system/fluent-research-notes.md`
- 핵심 산출물(적용 스키마): `docs/design-system/MS_FLUENT_TOKEN_SCHEMA.md`

## 핵심 규격 정리 (요약)
- 디자인 토큰 2단계 구조
  - 글로벌 토큰(원시값/단위) + 앨리어스 토큰(의미 레이어).
  - 색/타이포/테두리/애니메이션까지 토큰으로 관리.
- 타이포
  - Fluent 웹/윈도우/모바일 별 텍스트 램프를 제공.
  - 버튼/기본 텍스트/타이틀 사이즈를 레벨 단위로 계층화.
- 레이아웃/간격
  - 0,2,4,6,8,10...로 시작하는 spacing ramp 존재 (`size20` = 2, `size40` = 4, `size560` = 56).
  - 모바일 터치 타깃 권장치: iOS/Web 44px, Android 48px.
  - 12컬럼 그리드와 region/margin/gutter 접근 권장.
- 색상
  - neutral/shared/brand/semantic 계열을 구분해 사용.
  - 인터랙션 상태는 기본적으로 `rest -> hover -> pressed/selected`로 진화(플랫폼별 변형 존재).
  - 컨트랙스트/브랜드 색은 과도 사용 지양.
- Motion
  - 목적성/자연스러움/일관성 3원칙.
  - Enter/Exit, Elevation, Top-level, Container transform 등 전환 패턴 존재.
  - WCAG 권장 “no motion” 지원, 과도한 깜빡임 금지, 동작은 포커스 요소 중심.
- 접근성/텍스트/포커스
  - WCAG AA 기준 준수 지향: 일반 텍스트 4.5:1, 큰 텍스트 3:1.
  - 포커스 관리(논리적 흐름), 키보드 경로, 대체 텍스트, 의미 있는 텍스트 작성 필요.
- 아이콘
  - System / Product launch / File icon 분기.
  - 12px은 정보성 표시 위주(상호작용 전용으로 쓰지 않기).
  - Product launch 아이콘 크롭/확장 규칙(스케일 팩터 준수).
- 고대비(Windows)
  - forced-colors 미디어 쿼리 대응.
  - `SystemColors` 기반 매핑을 사용해 HC 텍스트/배경 대비 보강.
  - 10:1 요구 테이블 기반 색 조합 검증 권장.

## 코드 반영 우선순위 체크리스트

### 1) 토큰 우선 정리 (필수)
- [ ] 전역 디자인 토큰 사전 작성: 색/타이포/간격/코너/stroke/focus.
- [ ] 토큰 이름을 플랫폼별로 고정(예: Fluent 표준 토큰명)하고, 하드코딩 색상/px를 제거.
- [ ] 다크/라이트/HC 테마에서 토큰 값이 분기되도록 확장.

### 2) 기본 아키텍처 정리 (필수)
- [ ] 컴포넌트를 Fluent 기반 컴포넌트 라이브러리로 묶는 공통 layer 추가.
  - Web Components: `provideFluentDesignSystem().register(...)`
  - React v9 시: `FluentProvider` + theme.
- [ ] 공통 spacing/typography/radius mixin을 기본 스타일 baseline으로 고정.
- [ ] 텍스트 기본은 `body` 텍스트 계열 + 계층형 스타일 적용만 허용.

### 3) 핵심 컴포넌트 토큰 바인딩 (중요)
- [ ] Button/Anchor/TextField/Card/Dialog/Dropdown/Badge 등 주요 컴포넌트별 토큰 매핑표 작성.
- [ ] 상태별 스타일(hover/focus/active/disabled/selected) 토큰 표준 적용.
- [ ] icon size/weight/placement 규칙을 텍스트 라인/버튼 높이와 정합.

### 4) 접근성 hardening (최우선)
- [ ] contrast(AA), aria-label/role, 키보드 포커스 순서, skip/heading 구조 점검.
- [ ] 동적 콘텐츠는 ARIA live로 상태 알림.
- [ ] interactive 요소에 최소 터치영역(44/48) 미달 여부 자동 검사.
- [ ] 텍스트 대체/설명 정책 적용.

### 5) 고대비/적응형 색상 (고우선)
- [ ] `@media (forced-colors: active)` 대응 코드 경로 확보.
- [ ] Windows HC 색 대응 토큰 맵(`CanvasText`, `ButtonText`, `Highlight` 등) 테스트.
- [ ] HC에서 hover/active/disabled/focus가 사라지지 않도록 fallback 스타일 보강.

### 6) Motion/transition 거버넌스
- [ ] transition 지속 시간/디퓨즈 패턴을 토큰화(짧고 예측 가능한 기본).
- [ ] "reduced motion/no-motion" 옵션 제공.
- [ ] 화면 전체를 흔드는 과도한 모션 제거, 컨테이너 내 집중 모션 적용.

### 7) 린트·테스트·자동검증
- [ ] 토큰 미사용/비표준 토큰 사용 탐지(`grep` 또는 stylelint 커스텀 룰).
- [ ] 시각 회귀 테스트(필요시 스냅샷/스토리북) + 콘트라스트 자동 검사.
- [ ] 접근성 감사 자동화(axe/Playwright + 키보드 시나리오).
- [ ] HC 모드 수동 Smoke check 필수(High Contrast Black/White 기본 프로필).

### 8) 설정 UX 가드레일 (오입력 방지)
- [ ] 설정 항목을 `필수`와 `선택`으로 구분하고, 필수는 라벨에서 즉시 식별 가능.
- [ ] 저장 버튼은 필수 입력의 형식 검증이 완료될 때까지 비활성화.
- [ ] 검증 상태는 화면 1곳(상태 배지)에서만 보여주고, 중복 에러 문구를 피함.
- [ ] 선택형 항목은 기본값을 명시하고, 클릭 가능한 칩/세그먼트 등 저마찰 입력으로 제공.
- [ ] 위험 액션(재동기화/삭제/배포)은 별도 박스 + 확인 체크/문구 입력 후에만 실행 가능.
- [ ] endpoint/token/path 등 오입력 빈도가 높은 필드는 예시 placeholder + 형식 힌트 + 실시간 검사 제공.

## 즉시 실행용 체크리스트 (개발자 입력용)
- [ ] `MS_FLUENT_TOKEN_SCHEMA.md` 파일 생성(토큰 정의 + 사용 규칙).
- [ ] `docs/design-system/fluent-theme.css` 또는 theme provider에서 공통 토큰 주입.
- [ ] 버튼/입력/카드/링크 우선 리팩터링 → 이후 폼/리스트/모달.
- [ ] 접근성/고대비/동작 애니메이션 수동 체크리스트를 CI 파이프라인에 한 단계로 연결.

## 참고: 현재 저장소 적용 범위
- 본 저장소는 현재 CLI/러스트 중심으로 UI 코드가 거의 없으므로, 위 항목은 향후 프론트엔드(예: 위젯/앱/확장 UI) 추가 시 적용 대상으로 두고,  
  지금은 `docs/design-system/ms_design_checklist_fluent.md`를 정책 기반 기준 문서로 남기면 된다.

## 실행 산출물
- [ ] `docs/design-system/MS_FLUENT_TOKEN_SCHEMA.md`로 스키마 정합 검토
- [ ] `docs/design-system/fluent-tokens.json`로 실제 토큰 데이터 등록
- [ ] `docs/design-system/fluent-theme.css`로 토큰 주입 + 컴포넌트 기본 클래스 반영
- [ ] `docs/design-system/fluent-wrappers.tsx`로 Button/Input/Card/Dialog 래퍼 1차 PR
