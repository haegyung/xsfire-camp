# Release/Registry Unblock Checklist (v0.9.15)

## Goal
`v0.9.15`의 마지막 외부 차단 2건(npm publish, Zed registry merge)을 닫아 실제 배포 반영을 완료한다.

## Current Blockers
1. npm registry 미반영
- 증상: `npm view @haegyung/xsfire-camp` -> `E404`
- 근거: release workflow run `22294528645` 실패 로그에 `ENEEDAUTH`, `NPM_TOKEN` empty

2. Zed registry PR 미병합
- PR: `https://github.com/zed-industries/extensions/pull/4811`
- 상태: `OPEN`, `mergeStateStatus=BLOCKED` (merge queue + maintainer 권한 필요)

## Checklist A: npm Publish 복구

### A-1. 옵션 결정
다음 중 하나를 선택:
1. Trusted Publishing (OIDC) 사용
2. `NPM_TOKEN` repository secret 사용

### A-2. Trusted Publishing 경로
1. npm 패키지/조직 설정에서 `haegyung/xsfire-camp` GitHub Actions를 trusted publisher로 등록
2. workflow의 `id-token: write` 권한은 이미 있음 (`.github/workflows/release.yml`)
3. 설정 후 `release.yml` 재실행
4. 성공 확인:
```bash
npm view @haegyung/xsfire-camp version
```

### A-3. NPM_TOKEN 경로
1. npm에서 publish 권한 토큰 발급
2. GitHub repo secret `NPM_TOKEN` 등록
3. `release.yml` 재실행
4. 성공 확인:
```bash
npm view @haegyung/xsfire-camp version
```

## Checklist B: Zed Registry 병합
1. PR #4811 유지 (이미 `v0.9.15` 반영됨)
2. maintainer가 merge queue로 enqueue/merge
3. 병합 확인:
```bash
gh pr view 4811 --repo zed-industries/extensions --json state,mergedAt,url
```

## Verification Commands
```bash
gh run list --repo haegyung/xsfire-camp --workflow release.yml --limit 3
npm view @haegyung/xsfire-camp version
gh pr view 4811 --repo zed-industries/extensions --json state,mergeStateStatus,url
```

## Done Criteria
1. npm 패키지 버전 조회 성공
2. Zed PR merged 상태 확인
3. 이 문서의 Blockers 섹션을 "resolved"로 업데이트
