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
| `rust-v<upstream>-ori.N` | 우리 릴리스 태그 — `orihouse-release` workflow 가 upstream 과 같은 이름의 musl 자산을 만든다 |

## 원칙

1. 코드 수정은 최소로 한다. 기존 파일에는 몇 줄만 끼우고, 새 코드는 새 파일에 둔다. 지우지 않는다.
2. 가볍게 하는 것은 먼저 칸 설정(`-c features.<f>=false`)으로 한다. 설정으로 안 꺼지는 것만 여기 커밋으로 끈다.
   칸이 끄는 목록 = orihouse_ws `exit_gate_v2/codex_host_data.py` `CODEX_FEATURES_OFF`.
3. 커밋 하나 = 기능 하나. 커밋 메시지 머리에 `orihouse:` 를 붙인다.

## 커밋 묶음 (v0.166.0)

| # | 커밋 | 거는 자리 |
|---|---|---|
| 0 | 이 문서 | `ORIHOUSE.md` |
| 1 | compact 전용 모델 `compact_model` · `compact_reasoning_effort` | `config/src/config_toml.rs` · `core/config.schema.json` · `core/src/config/mod.rs`(필드 2 · 대입 2) · `core/src/session/mod.rs`(mod 1줄) · `core/src/compact_remote_v2.rs`(수동·자동 진입점 2곳) · 새 파일 `core/src/session/orihouse_compact.rs` · 시험 `core/tests/suite/orihouse_compact_model.rs` |
| 2 | musl 릴리스 workflow | `.github/workflows/orihouse-release.yml` (GitHub API 로 올림 — 아래 함정) |

커밋 #1 의 동작: 압축 요청 한 번만 압축 모델·effort 로 만든 step 으로 보낸다. 교체 창과 다음 턴은 스레드 모델이고,
압축 모델 요청이 실패하면 upstream fallback 이 스레드 모델로 다시 압축한다. upstream 의 이전 모델 압축(모델을 바꾼 턴)과
백그라운드 압축 후보는 건드리지 않는다. 지원하지 않는 effort 면 경고 한 줄 뒤 스레드 설정으로 압축한다.

## 시험 (Linux — Windows 는 rusty_v8 git 체크아웃이 260자 경로 제한에 걸려 컴파일 불가)

```
docker run --rm -v <이 저장소>:/src:ro -v codex-fork-work:/work -v codex-fork-cargo:/usr/local/cargo/registry \
  -v codex-fork-git:/usr/local/cargo/git rust:1.95-bookworm sh -c '
  apt-get update -qq && apt-get install -y -qq libcap-dev pkg-config clang git
  git -c safe.directory="*" clone -q /src /work/codex && cd /work/codex/codex-rs
  cargo fmt --check -p codex-core -p codex-config
  RUST_MIN_STACK=8388608 CARGO_TARGET_DIR=/work/target cargo test -p codex-core --test all orihouse_compact'
```

- `RUST_MIN_STACK=8388608` 없으면 debug 시험이 stack overflow 로 죽는다(upstream `just test` 와 같은 값).
- `--test all compact` 를 돌리면 code_mode · guardian 압축 시험 18건이 실패한다. upstream 원본(커밋 #0)에서도 같은 18건이
  실패한다 — `codex-code-mode-host` 형제 바이너리가 이 환경에서 안 만들어지기 때문이다. 우리 회귀 판정은 이 18건을 뺀 나머지로 한다.

## upstream 올리기

```
git fetch upstream --tags
git switch -c orihouse/v<new> rust-v<new>
git cherry-pick <orihouse/v<old> 의 우리 커밋들>      # 또는 git rebase --onto rust-v<new> rust-v<old> orihouse/v<old>
(위 Linux 시험)
git push origin orihouse/v<new>        # 원격 브랜치는 먼저 API 로 rust-v<new> 커밋에 만들어 둔다(아래 함정)
gh api -X POST repos/OriHouse/codex/git/refs -f ref=refs/tags/rust-v<new>-ori.1 -f sha=<커밋>
```

그 뒤 orihouse_ws 쪽: app-server schema diff 게이트(`evidence/loop3/schema`) → strict-config 프로브
(`codex --strict-config -c compact_model="…" app-server` 가 initialize 에 답하는지) → 칸 Dockerfile `CODEX_VERSION` → dev 칸 하나 → 전 칸.

## 함정

- **git push 가 workflow 파일을 담으면 GitHub 가 거절한다**(`Unable to determine if workflow can be created … timeout` ·
  `Internal Server Error`). 이 저장소는 workflow 가 많아 권한 검사가 시간 안에 안 끝난다. workflow 파일은
  `gh api -X PUT repos/OriHouse/codex/contents/<경로>` 로 올리고, 새 브랜치는 `git/refs` API 로 이미 있는 커밋에 먼저 만든 뒤
  나머지 커밋만 push 한다. 0.166.0 의 커밋 #2 가 API 커밋 두 개(빈 파일 → 내용)인 것은 이 과정의 실수 흔적이다.
- **fork 의 upstream workflow 는 꺼 둔다**(28개 `disabled_manually`). 태그 `rust-v*` 에 upstream `rust-release` 가 반응한다.
- 일반판 codex 는 `compact_*` 설정을 `unrecognized … ignored` 경고로 무시한다. 칸이 일반판으로 되돌아가면 이 경고가 보인다.
