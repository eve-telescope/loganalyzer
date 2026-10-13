# Combat Log Analyzer

[![CI](https://github.com/eve-telescope/loganalyzer/actions/workflows/ci.yml/badge.svg)](https://github.com/eve-telescope/loganalyzer/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Website](https://img.shields.io/website?url=https%3A%2F%2Floganalyzer.eve-telescope.com&label=loganalyzer.eve-telescope.com)](https://loganalyzer.eve-telescope.com)
[![Rust](https://img.shields.io/badge/Rust-Axum-DEA584?logo=rust&logoColor=white)](https://github.com/tokio-rs/axum)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-Svelte%205-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/docs/kit)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-18-4169E1?logo=postgresql&logoColor=white)](https://www.postgresql.org)

Upload an EVE Online combat log and get an after-action report: DPS over
time, damage breakdowns, logistics, energy warfare and the numbers per
pilot, at a link you can share. Part of the Telescope toolkit, live at
https://loganalyzer.eve-telescope.com.

Your gamelogs are in `Documents/EVE/logs/Gamelogs`. Drop one on the page
and share the link it takes you to.

## Features

- Drag and drop upload of a gamelog (`.txt` or `.log`, up to 25 MB),
  checked in the browser before it is sent, with upload progress
- A parser for the combat lines of the English client: hits and misses
  both ways, remote shield, armor and hull repairs, neutralizers and
  nosferatu, drone damage attributed to its owner where the log tells
- Pilot portraits, ship icons and zKillboard links, with the names
  resolved through ESI and cached
- A timeline of DPS, repairs and energy warfare; drag over it to look at
  a time range
- Totals, peaks, hit quality, damage by target, source and weapon, and a
  per-pilot engagement log
- Shareable links that keep the selected range, the hidden series and
  the open pilot, with link previews that name the pilot and the totals
- The raw log for download, and a fresh analysis of it after parser
  improvements

## Stack

- API: Rust, [Axum](https://github.com/tokio-rs/axum) and
  [sqlx](https://github.com/launchbadge/sqlx) on Postgres; the parser
  runs on [PCRE2](https://github.com/BurntSushi/rust-pcre2)
- Frontend: [SvelteKit](https://svelte.dev/docs/kit) (SSR,
  adapter-node), Svelte 5, Tailwind CSS 4 and
  [Chart.js](https://www.chartjs.org)
- Names from [ESI](https://developers.eveonline.com/api-explorer)
  (`POST /universe/ids/`), images from images.evetech.net

One origin serves both: Caddy (or the Vite dev proxy) sends `/api/*`,
`/up`, `/analyze`, the raw log download and the regeneration
(`/logs/*/download`, `/logs/*/regenerate`) and every request that is not
a GET or HEAD to Axum, and page loads to SvelteKit.
`frontend/proxy-paths.ts` and `deploy/Caddyfile` hold that list.

## Running it

The whole stack (Postgres, API, SvelteKit, Caddy) on
http://localhost:5317, with three sample logs stored on start (at
`/logs/5a3b6e1c-0d4f-4a2b-8c7d-1e2f3a4b5c01` to `...5c03`):

```bash
docker compose up --build -d
```

Or natively, with only Postgres in docker:

```bash
docker compose up -d postgres     # Postgres on 127.0.0.1:5447
cargo run                         # API on 127.0.0.1:3000
cd frontend && npm install && npm run dev
```

Configuration lives in `.env` (see `.env.example`); everything has a
working local default. `cargo run --bin seed_samples` stores the sample
logs in the native setup.

## Testing

```bash
docker compose up -d postgres
cargo test
cd frontend && npm run check && npm run lint && npm run format:check && npm test
docker compose up --build -d && cd frontend && npm run test:e2e
```

The Rust suites drive the real router on their own `loganalyzer_test`
database, with ESI replaced by a mock on an ephemeral port. The parser
is pinned over every fixture log in `tests/fixtures/parser` and a corpus
of generated ones. The vitest suites check the analysis math value by
value, and Playwright runs against the composed stack on port 5317.

## Deployment

Coolify runs `compose.prod.yml`. A push to main runs
`.github/workflows/release.yml`, which builds the api, frontend and
proxy images, pushes them to `ghcr.io/eve-telescope/loganalyzer-*`
(`latest` and the commit) and calls Coolify's deploy webhook, so Coolify
pulls and restarts them. Only `loganalyzer-proxy` has the domain;
Traefik terminates TLS in front of it and the proxy routes with
`deploy/Caddyfile`, the same file the local stack mounts. The api
reaches the shared Postgres over the `coolify` network (`DATABASE_URL`
is a Coolify environment variable), runs its migrations on startup and
keeps the raw logs on the `loganalyzer_storage` volume.

## Layout

```
src/                 the API: parser, ESI client, uploads, pages' props, router
src/parser/          the gamelog parser and its text helpers
src/bin/             migrate, seed_samples
migrations/          the Postgres schema
tests/integration/   the Rust suites, one binary
tests/fixtures/      gamelogs with their parsed events, the ESI name table
frontend/            the SvelteKit app and its Playwright suite (e2e/)
deploy/              the proxy image and its Caddyfile (local and production)
```

## License

Open-source software licensed under the
[MIT license](https://opensource.org/licenses/MIT).
