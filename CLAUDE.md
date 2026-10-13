# loganalyzer.eve-telescope.com

The Combat Log Analyzer: anyone uploads an EVE Online gamelog, the api
parses it into events, stores them with the gzipped raw file, and the
analysis page (`/logs/{uuid}`) computes every number and chart in the
browser from the event list. No accounts, no mail, no queue. A Rust API
(Axum + Postgres via sqlx) and a SvelteKit frontend (SSR, adapter-node,
Svelte 5, Tailwind 4, chart.js) in `frontend/`. One shared origin (the
Vite dev proxy locally, Caddy in docker and production) routes backend
paths and every mutation to Axum and page GETs to SvelteKit. Never point
the browser at Axum's port directly. The setup mirrors Medico
(`../Medico`) and body-and-face (`../body-and-face`): when in doubt about
structure or tooling, do what they do. ESI work follows MutaMarket
(`../MutaMarket`).

## The legacy reference

The site ran as a Laravel 13 + Inertia/Vue app (Octane), which is still on
the `main` branch until cutover (and checked out read-only at
`../forge-migration/legacy/loganalyzer`). It is the spec for behavior:
read how it does something before changing or completing a feature here,
and mirror it. The plan is
`../forge-migration/sites/loganalyzer.eve-telescope.com/plan.md`.

- `tests/fixtures/golden/` holds what Laravel actually answered, recorded
  with `scripts/laravel-reference/` (`run.sh setup`, `run.sh start`,
  `record.py`, `capture.mjs`, `analysis-outputs.mjs`), against the mock
  ESI table `tests/fixtures/esi/names.json`.
- `tests/fixtures/parser/*.expected.json` and `generated/expected.json`
  are the PHP parser's answers (`parser_expected.py`),
  `tests/fixtures/php-functions/answers.json` PHP's string functions'.
- Never regenerate a fixture to make a failing change pass.

Every Laravel bug is fixed, not copied, and logged in `FIXES.md` with the
test that pins it. Any other difference from Laravel is a bug here.

## The parser

`src/parser/` keeps PHP's `preg_*` patterns as they were, on PCRE2 with
PHP's `/u` options (UTF and UCP), so its output matches the PHP parser
byte for byte; `strip_tags`, `mb_trim` and `(int)` are ported in
`src/parser/php.rs`. A change to the parser needs `cargo test parser::`
and `scripts/parser-diff` (PHP in the reference container against
`src/bin/parse_logs.rs`) to stay green. Pattern strings with `\u` must be
written with escaped backslashes.

## Testing rules

- Every change lands with its tests in the same commit.
- Tests are precise: exact JSON key sets at every level
  (`common::assert_json_eq`), exact messages, statuses and orderings.
- Behavior tests drive the real router (`oneshot`) on the dedicated
  `loganalyzer_test` database (`db::test_pool()`, created and migrated on
  first use, `TEST_DATABASE_URL` to override). ESI is a local mock on an
  ephemeral port (`tests/integration/common/esi.rs`), never a stub of our
  own code.
- Integration suites are modules of the single `tests/integration` binary
  and run single-threaded (`.cargo/config.toml`): they truncate the shared
  tables. Run one suite with `cargo test <module>::`.
- Frontend logic has vitest suites next to the code (`*.test.ts`; the
  `*.svelte.test.ts` ones run in Chromium). Playwright (`frontend/e2e`)
  runs against the composed stack and seeds through
  `docker exec loganalyzer-postgres psql`.
- `tests/PORTED.md` maps every PHP test to its port; `scripts/parity`
  prints passing over total per suite.
- Before committing: `cargo fmt`, `cargo clippy --all-targets` (zero
  warnings), `cargo test`, and in `frontend/` `npm run check`,
  `npm run lint`, `npm run format:check`, `npm test`.

## Layout and commands

- `src/server/` the router and the layers; `src/logs.rs` storing,
  regenerating and reading logs; `src/entities.rs` and `src/esi.rs` the
  name cache and the ESI client; `migrations/` the schema; `deploy/` the
  Caddyfiles; `frontend/proxy-paths.ts` the routing contract, mirrored by
  `deploy/Caddyfile.dev` and `deploy/Caddyfile`.
- Development: `docker compose up -d postgres`, `cargo run` (API on
  `BIND_ADDR`, default 127.0.0.1:3000, see `.env.example`) and
  `npm run dev` in `frontend/`.
- Whole stack: `docker compose up --build` on http://localhost:5317.
  Ports are fixed: Postgres 5447, stack 5317, the Laravel reference 8317.
  The `sample-seed` service stores three sample logs (fixed uuids
  `5a3b6e1c-0d4f-4a2b-8c7d-1e2f3a4b5c01` to `...03`, names resolved
  through the real ESI). Playwright truncates the same database and
  caches the mock's ids, so reset the stack after an e2e run
  (`docker compose down -v`, then up again).
- Local preview on https://loganalyzer.test (portal):
  `APP_URL=https://loganalyzer.test ORIGIN=https://loganalyzer.test
  docker compose up --build -d`.
- Frontend: pages load their props from `/api/pages/<path>`
  (`#lib/server/api.ts`); the analysis math is `#lib/analysis.ts`, lifted
  from the Laravel page and pinned value by value.
