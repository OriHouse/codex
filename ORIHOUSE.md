# OriHouse codex fork

orihouse-code 가 GPT 에 기대는 경로(exit-gate-v2 칸의 `codex app-server`)에 쓰는 codex 다.
upstream(`openai/codex`) 태그 위에 작은 커밋 묶음을 rebase 로 쌓아 유지한다.

- spec: OriHouse/orihouse_ws `docs/users/tkdgur4427/109_exit_gate_v2/loop9_codex_fork/spec.md`
- 라이선스: upstream 과 같다(Apache-2.0 · `LICENSE` · `NOTICE` 그대로).

## 브랜치와 태그

| 이름 | 뜻 |
|---|---|
| `orihouse/v<upstream>` | upstream 태그 `rust-v<upstream>` 위에 우리 커밋 묶음 |
| `orihouse/main` | 지금 칸이 쓰는 `orihouse/v<upstream>` 과 같은 커밋(기본 브랜치) |
| `rust-v<upstream>-ori.N` | 우리 릴리스 태그 — CI 가 upstream 과 같은 이름의 musl 자산을 만든다 |

## 원칙

1. 코드 수정은 최소로 한다. 기존 파일에는 몇 줄만 끼우고, 새 코드는 새 파일에 둔다. 지우지 않는다.
2. 가볍게 하는 것은 먼저 칸 설정(`-c features.<f>=false`)으로 한다. 설정으로 안 꺼지는 것만 여기 커밋으로 끈다.
3. 커밋 하나 = 기능 하나. 커밋 메시지 머리에 `orihouse:` 를 붙인다.

## 커밋 묶음

| # | 커밋 | 거는 자리 |
|---|---|---|
| 0 | 이 문서 | `ORIHOUSE.md` |
| 1 | (T3) compact 전용 모델 `compact_model` · `compact_reasoning_effort` | config_toml · core config · turn_context · compact 진입점 |
| 2 | (T4) musl 한 줄 릴리스 workflow | `.github/workflows/orihouse-release.yml` |

## upstream 올리기

```
git fetch upstream --tags
git switch -c orihouse/v<new> rust-v<new>
git cherry-pick <orihouse/v<old> 의 우리 커밋들>      # 또는 git rebase --onto rust-v<new> rust-v<old> orihouse/v<old>
cargo test -p codex-core compact_model_            # 우리 커밋 시험
git tag rust-v<new>-ori.1 && git push origin orihouse/v<new> rust-v<new>-ori.1
```

그 뒤 orihouse_ws 쪽에서 app-server schema diff 게이트와 strict-config 프로브를 돌리고, dev 칸 하나 → 전 칸 순서로 올린다.
