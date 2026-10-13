// What the Laravel app's analysis page computed: the <script setup> of
// resources/js/pages/Analysis.vue (and of the components with logic of
// their own) from `main`, run in node with Vue's reactivity replaced by
// plain getters, over the props the reference served
// (tests/fixtures/golden/pages) and the URL states of
// tests/fixtures/golden/analysis/cases.json. Writes
// tests/fixtures/golden/analysis/<case>.json, which
// frontend/src/lib/analysis.test.ts compares the port with.
//
//   cd frontend && TZ=UTC LC_ALL=en_US.UTF-8 node ../scripts/laravel-reference/analysis-outputs.mjs
//
// The clock is fixed at 2026-10-13T12:00:00Z for the relative times.
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(new URL('../../frontend/package.json', import.meta.url));
const ts = require('typescript');

const REPO = new URL('../../', import.meta.url);
const OUT = new URL('tests/fixtures/golden/analysis/', REPO);
const NOW = Date.UTC(2026, 9, 13, 12, 0, 0);

function legacy(path) {
  return execFileSync('git', ['-C', REPO.pathname, 'show', `main:${path}`], { encoding: 'utf8' });
}

function scriptOf(sfc) {
  return sfc.match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1];
}

/** A script setup as a function of its props, returning its bindings. */
function setupFunction(source, exported, prelude = '') {
  const withoutImports = source.replace(/^import[\s\S]*?from\s+'[^']+';\s*$/gm, '');
  const js = ts.transpileModule(prelude + withoutImports, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  }).outputText;
  const body = js
    .replace(/\bexport\s+/g, '')
    .replace(/\bdefineProps\(/g, '__defineProps(')
    .replace(/\bwithDefaults\(\s*__defineProps\(\)\s*,/g, '__withDefaults(')
    .replace(/\bdefineEmits\(/g, '__noop(')
    .replace(/\bcomputed\(/g, '__computed(')
    .replace(/\bshallowRef\(/g, '__ref(')
    .replace(/\bref\(/g, '__ref(')
    .replace(/\bwatch\(/g, '__noop(')
    .replace(/\bonMounted\(/g, '__noop(')
    .replace(/\bonBeforeUnmount\(/g, '__noop(')
    .replace(/\buseForm\(/g, '__useForm(')
    .replace(/\bChartJS\.register\(/g, '__noop(');
  return new Function(
    '__props',
    `const __defineProps = () => __props;
     const __withDefaults = (defaults) => ({ ...defaults, ...__props });
     const __noop = () => {};
     const __computed = (fn) => ({ get value() { return fn(); } });
     const __ref = (value) => ({ value });
     const __useForm = () => ({ processing: false, submit() {} });
     const window = undefined;
     ${body}
     return { ${exported.join(', ')} };`,
  );
}

const dates = legacy('resources/js/lib/dates.ts');
const analysis = setupFunction(
  scriptOf(legacy('resources/js/pages/Analysis.vue')),
  [
    'ogTitle', 'ogDescription', 'selection', 'hiddenSeries', 'dpsOverTime', 'filteredEvents',
    'summary', 'details', 'peakDps', 'tankSupportRatio', 'neutPressureIn', 'neutPressureOut',
    'qualitySegmentsOut', 'qualitySegmentsIn', 'damageByTarget', 'damageByWeapon',
    'incomingBySource', 'incomingByWeapon', 'outgoingTableRows', 'incomingTableRows',
    'logiTableRows', 'damageByTargetItems', 'damageByWeaponItems', 'incomingBySourceItems',
    'incomingByWeaponItems', 'selectionLabel', 'selectedPilot', 'engagement', 'engagementLog',
    'formatDuration', 'formatNumber', 'relativeTimeFromEve',
  ],
  dates,
);
const avatar = setupFunction(scriptOf(legacy('resources/js/components/ui/PilotAvatar.vue')), [
  'initials', 'hueClass', 'portraitUrl',
]);
const spectrum = setupFunction(
  scriptOf(legacy('resources/js/components/charts/QualitySpectrum.vue')),
  ['total', 'items', 'visibleItems'],
);

const realNow = Date.now;
Date.now = () => NOW;

const unwrap = (value) =>
  value instanceof Set
    ? [...value]
    : value && typeof value === 'object' && 'value' in value && Object.keys(value).length === 1
      ? unwrap(value.value)
      : value;

const cases = JSON.parse(readFileSync(new URL('cases.json', OUT), 'utf8'));
mkdirSync(OUT, { recursive: true });
for (const [name, { page, filters }] of Object.entries(cases)) {
  const recorded = JSON.parse(
    readFileSync(new URL(`tests/fixtures/golden/pages/${page}.json`, REPO), 'utf8'),
  );
  const props = { ...recorded.page.props, filters: { ...recorded.page.props.filters, ...filters } };
  const bindings = analysis(props);
  const result = {};
  for (const [key, value] of Object.entries(bindings)) {
    if (typeof value === 'function') {
      continue;
    }
    result[key] = unwrap(value);
  }
  result.filteredEventCount = result.filteredEvents.length;
  delete result.filteredEvents;
  result.durations = [0, 1, 59, 60, 61, 3599, 3600, 7322].map((s) => bindings.formatDuration(s));
  result.relativeSessionStart = bindings.relativeTimeFromEve(props.analysis.sessionStarted);
  // The avatars of every pilot on the page, and the spectrum's segments.
  const names = [...new Set(props.analysis.events.map((event) => event.playerName))];
  result.avatars = Object.fromEntries(
    names.map((pilot) => {
      const avatarBindings = avatar({ name: pilot, characterId: props.pilotIds[pilot] ?? null });
      return [pilot, { initials: unwrap(avatarBindings.initials), hue: unwrap(avatarBindings.hueClass), portrait: unwrap(avatarBindings.portraitUrl) }];
    }),
  );
  result.spectrumOut = unwrap(spectrum({ segments: result.qualitySegmentsOut, hue: 'cyan' }).visibleItems);
  result.spectrumIn = unwrap(spectrum({ segments: result.qualitySegmentsIn, hue: 'red' }).visibleItems);
  writeFileSync(new URL(`${name}.json`, OUT), `${JSON.stringify(result, null, 2)}\n`);
  console.log(`recorded ${name}`);
}
Date.now = realNow;
