# MS Fluent Design Kit (Bootstrap Pack)

이 폴더는 현재 CLI 중심 리포지토리에 프런트엔드가 없더라도, 향후 MCP 위젯/UI를 붙일 때 바로 쓰는 `MS Fluent` 기반의
최소 토큰/테마/컴포넌트 뼈대 산출물이다.

## 파일 구성

- `MS_FLUENT_TOKEN_SCHEMA.md`: 스키마 정의(필수 체크포인트 포함)
- `fluent-tokens.json`: 샘플 토큰 데이터(경량 예시)
- `fluent-theme.css`: CSS custom-property 기반 토큰 주입 + 기본 컴포넌트 래퍼 클래스
- `fluent-wrappers.tsx`: React용 Button/Input/Card/Dialog 래퍼 예시

## 적용 순서

1. `fluent-theme.css`를 위젯/앱 번들에 포함
2. 루트에 `data-ms-theme="light|dark|highContrast"`를 설정하고 `.ms-fluent-root` 래퍼로 감싸기
3. 기존 컴포넌트를 `MsButton`, `MsInput`, `MsCard`, `MsDialog`로 교체
4. 토큰 변경 시 `fluent-tokens.json`을 갱신하고 CSS 빌드 단계에 반영
5. `docs/` 외부에서 쓰는 경우:
   - 고대비 환경 강제 토글(`forced-colors`) 케이스만 먼저 수동 검증
   - `prefers-reduced-motion` 경로가 깨지지 않는지 확인

## 사용 예시

```tsx
import { MsFluentTheme, MsButton, MsInput, MsCard, MsDialog } from './design-system/fluent-wrappers';
import './design-system/fluent-theme.css';

export function Demo() {
  const [open, setOpen] = React.useState(false);
  return (
    <MsFluentTheme theme="light">
      <MsCard title="Design Token Trial">
        <MsInput label="Email" placeholder="you@example.com" />
        <MsButton onClick={() => setOpen(true)}>Open Dialog</MsButton>
      </MsCard>
      <MsDialog open={open} onClose={() => setOpen(false)} title="Hello">
        Fluent 스타일 토큰이 적용된 다이얼로그
      </MsDialog>
    </MsFluentTheme>
  );
}
```

## 런타임 점검 체크

- [ ] `ms-fluent-button` 터치 높이 44/48 충족
- [ ] 키보드 포커스 링이 `--ms-focus-*` 토큰을 따름
- [ ] 다크/라이트/고대비 전환 시 `data-ms-theme`만 바뀌어도 레이아웃이 깨지지 않음
- [ ] 버튼/인풋 기본 인터랙션이 토큰 상태값(`hover/pressed/disabled`)으로만 제어됨
