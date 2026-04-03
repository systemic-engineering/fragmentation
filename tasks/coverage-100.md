# Coverage: Achieve 100% line coverage

**Priority:** High — TDD gate is non-negotiable.
**Current:** 75.4% line coverage (lib only).
**Target:** 100%

## Gaps

Run `nix develop -c cargo llvm-cov --lib` to see current state.

Known uncovered modules:
- `visibility.rs` — 0% (Public/Protected/Private wrapper types)
- `walk.rs` — 0% (tree traversal: collect, fold, find, depth)
- `bounded_store.rs` — GitBoundedStore git read path
- `git.rs` — several read/write paths
- `fuse.rs` — FUSE filesystem (may need feature-gated coverage)
- `supervision.rs` — SupervisionTree

## Approach

1. Run `cargo llvm-cov --lib --html` to get visual report
2. For each uncovered module, write tests that exercise the public API
3. Use the existing test patterns (tempdir, MockHash, etc.)
4. Coverage ignore comments only for truly unreachable code (document why)

## Rules

- 100% line coverage. No cheating.
- `coveralls-ignore` only for genuinely unreachable paths (with comment explaining why)
- If code can't be tested, refactor until it can
