# rust-url vs ada benchmark harness

Compares this checkout of `url` against the C++ [ada](https://github.com/ada-url/ada)
parser, via the `ada-url` Rust bindings, over the same inputs in one process.

Neither crate is a member of the root workspace, so no `ada-url` dependency
reaches the `url` crate's own manifest. Both build with `lto = true` and
`codegen-units = 1`, so neither parser is measured with a codegen advantage the
other did not get.

## Setup

The corpus is ada's own published dataset (~8.8 MB, not committed):

```sh
cd benchmarks/ada-comparison
curl -o top100.txt \
  https://raw.githubusercontent.com/ada-url/url-various-datasets/main/top100/top100.txt
```

Using ada's dataset rather than hand-picked inputs is deliberate — it avoids
choosing inputs that flatter either parser.

## Running

**Run the two arms serially, never concurrently.** These are single-threaded
latency measurements; two cargo processes competing for cores will distort both
arms, and not necessarily by the same amount.

**Measure a change with paired baselines, not against remembered numbers.**

```sh
git checkout main
cargo bench --bench compare -- --save-baseline clean

git checkout <your-branch>
cargo bench --bench compare -- --baseline clean
```

`cargo bench` builds with the `[profile.release]` settings above on both sides;
do not compare a run built with different optimization settings, and do not
compare against numbers from an earlier session on a differently-loaded machine.

**Keep the ada arms as a control.** ada's code is identical between the two
runs, so if its numbers move by more than ~1% between arms, the machine was not
quiet and the rust-url delta is not trustworthy either. This is the single most
useful validity check in the harness.

**Check the load average first.** A comparison taken under load produced a
confident false negative during this work — a change actually worth -17% read as
"no effect".

## What each benchmark measures

`compare.rs`:

- `parse_corpus` — bulk parse of every URL in the corpus.
- `parse_and_getters` — parse plus reading host/path/query, i.e. what a real
  caller does.
- `parse_single_short` — one short absolute URL; the `new URL()` hot path.
- `join_relative` — relative resolution. ada's only API re-parses the base on
  every call, so the fair comparison makes rust-url re-parse it too; the
  pre-parsed variant is reported separately because it is what a rust-url caller
  would actually write, and it is a real API advantage.
- `module_specifiers` — one base, five short relative specifiers; module
  resolution.
- `resolution_strategies` — re-parsing versus a caller-side cache. Not a
  rust-url change; recorded to show what a higher-level layer could save.

It also prints a **parse-divergence check** on startup: how many corpus URLs
each parser accepts. Both currently agree on all 100,025 inputs (99,999 accepted
by both, 26 rejected by both, zero divergence). **If that count ever changes, a
correctness regression has been introduced** — treat it as a failure, not noise.

`src/bin/allocs.rs` counts allocations and reallocations per parse via a
counting global allocator:

```sh
cargo run --release --bin allocs
```

`../ada-bisect` pins ada 3.4.6 — the last release before ada PR #1175 added its
whole-URL fast path — so that fast path's contribution can be measured on its
own. It is a separate crate because two versions of `ada-url` cannot coexist in
one build.
