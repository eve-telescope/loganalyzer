# Combat Log Analyzer

[![Tests](https://github.com/eve-telescope/loganalyzer/actions/workflows/tests.yml/badge.svg)](https://github.com/eve-telescope/loganalyzer/actions/workflows/tests.yml)
[![Code Quality](https://github.com/eve-telescope/loganalyzer/actions/workflows/code-quality.yml/badge.svg)](https://github.com/eve-telescope/loganalyzer/actions/workflows/code-quality.yml)

Upload and analyze **EVE Online** combat logs. Get DPS graphs, damage breakdowns, logistics statistics, and detailed per-pilot numbers from a plain gamelog file — shareable via a unique URL.

## Features

- **Drag & drop upload** of EVE Online gamelog files (size limit enforced client- and server-side, configurable via `LOGANALYZER_UPLOAD_MAX_SIZE_MB`)
- **DPS over time** chart with incoming/outgoing series
- **Damage breakdowns** by target, source, and weapon
- **Logistics tracking** — remote shield/armor/hull reps dealt and received
- **Energy warfare** — neutralizer and nosferatu events (incoming and outgoing)
- **Timeline filters** synced to the URL, so a filtered view can be shared
- **Shareable results** — every analyzed log gets a UUID-based URL

## Tech Stack

| Layer | Technology |
| --- | --- |
| Backend | [Laravel 13](https://laravel.com) (PHP 8.3+) |
| Frontend | [Vue 3](https://vuejs.org) + [Inertia.js v3](https://inertiajs.com) |
| Styling | [Tailwind CSS 4](https://tailwindcss.com) |
| Charts | [Chart.js](https://www.chartjs.org) via vue-chartjs |
| Routing bridge | [Laravel Wayfinder](https://github.com/laravel/wayfinder) (typed route functions for TypeScript) |
| Build | [Vite 8](https://vite.dev) |
| Database | SQLite (default) |

## Getting Started

### Requirements

- PHP 8.3+ with Composer
- Node.js 22+ with npm

### Setup

```bash
git clone https://github.com/eve-telescope/loganalyzer.git
cd loganalyzer
composer run setup
```

The `setup` script installs Composer and npm dependencies, creates the `.env` file, generates the application key, runs migrations, and builds the frontend.

### Development

```bash
composer run dev
```

This starts the Laravel dev server, queue worker, log tailing ([Pail](https://github.com/laravel/pail)), and Vite with hot module replacement — all in one terminal.

If you use [Laravel Herd](https://herd.laravel.com), the app is also served at `https://loganalyzer.test` — you only need `npm run dev` alongside it.

## Testing & Code Quality

```bash
php artisan test              # Pest test suite
composer lint                 # Laravel Pint (auto-fix)
composer lint:check           # Laravel Pint (check only)
vendor/bin/phpstan analyse    # Larastan static analysis
vendor/bin/rector --dry-run   # Rector automated refactoring (preview)
npm run lint                  # ESLint (auto-fix)
npm run format                # Prettier (auto-fix)
npm run types:check           # vue-tsc TypeScript check
composer ci:check             # Everything CI runs, locally
```

Test fixtures (synthetic gamelog files) live in `tests/Fixtures/`.

## Continuous Integration

GitHub Actions run on every push to `main` and on pull requests:

- **Tests** (`tests.yml`) — full Pest suite against PHP 8.3, 8.4, and 8.5
- **Code Quality** (`code-quality.yml`) — Pint, Larastan, Rector (dry run), ESLint, Prettier, and TypeScript checks

[Dependabot](.github/dependabot.yml) opens weekly grouped update PRs for Composer, npm, and GitHub Actions dependencies.

## How It Works

1. A gamelog file is uploaded and parsed by [`CombatLogParser`](app/Services/CombatLogParser.php), which extracts the listener (pilot), session start, and all combat events (damage, misses, logistics, energy warfare) via regex.
2. Events are stored as individual rows and the log gets a UUID.
3. The analysis page aggregates the events into charts and tables, entirely filterable by time range and event series.

## License

Open-source software licensed under the [MIT license](https://opensource.org/licenses/MIT).
