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

## The parser

`src/parser/` runs its patterns on PCRE2 in UTF and UCP mode;
`src/parser/text.rs` strips markup, trims and reads numbers. Every
fixture log in `tests/fixtures/parser` has its parsed output next to it
(`<name>.expected.json`), and `generated/` is a corpus of small logs with
theirs: `cargo test parser::` pins all of them, so a parser change that
alters an output updates that file in the same commit, on purpose.
Uploaded logs are kept, and a regeneration reparses them with the
current parser. Pattern strings with `\u` must be written with escaped
backslashes.

## Testing rules

- Every change lands with its tests in the same commit.
- Tests are precise: exact JSON key sets at every level
  (`common::assert_json_eq`), exact messages, statuses and orderings,
  with the expected values written out in the test.
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
- Before committing: `cargo fmt`, `cargo clippy --all-targets` (zero
  warnings), `cargo test`, and in `frontend/` `npm run check`,
  `npm run lint`, `npm run format:check`, `npm test`.

## Layout and commands

- `src/server/` the router and the layers; `src/logs.rs` storing,
  regenerating and reading logs; `src/entities.rs` and `src/esi.rs` the
  name cache and the ESI client; `migrations/` the schema; `deploy/` the
  proxy image and its Caddyfile (the local stack mounts the same file);
  `frontend/proxy-paths.ts` the routing contract, mirrored by
  `deploy/Caddyfile`.
- Development: `docker compose up -d postgres`, `cargo run` (API on
  `BIND_ADDR`, default 127.0.0.1:3000, see `.env.example`) and
  `npm run dev` in `frontend/`.
- Whole stack: `docker compose up --build` on http://localhost:5317.
  Ports are fixed: Postgres 5447, stack 5317.
  The `sample-seed` service stores three sample logs (fixed uuids
  `5a3b6e1c-0d4f-4a2b-8c7d-1e2f3a4b5c01` to `...03`, names resolved
  through the real ESI). Playwright truncates the same database and
  caches the mock's ids, so reset the stack after an e2e run
  (`docker compose down -v`, then up again).
- Local preview on https://loganalyzer.test (portal):
  `APP_URL=https://loganalyzer.test ORIGIN=https://loganalyzer.test
  docker compose up --build -d`.
- Frontend: pages load their props from `/api/pages/<path>`
  (`#lib/server/api.ts`); the analysis math is `#lib/analysis.ts`, pinned
  value by value in `analysis.test.ts`.
- Production: Coolify runs `compose.prod.yml` (`loganalyzer-api`,
  `loganalyzer-frontend` and `loganalyzer-proxy`, images from GHCR built
  by `.github/workflows/release.yml` on every push to main); the api
  reaches the shared Coolify Postgres over the `coolify` network (see the
  README).
