<script lang="ts">
  import { invalidateAll, replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';

  import {
    ENGAGEMENT_TYPES,
    ENGAGEMENT_TYPE_LABELS,
    accuracyValue,
    buildDamageByEntity,
    buildDetails,
    buildSummary,
    buildWeaponBreakdown,
    dpsOverTime as bucketDps,
    engagementLog as logOf,
    engagementOf,
    entityItems,
    eventColorClass,
    filterEvents,
    formatDuration,
    formatNumber,
    gjFormat,
    gjPerSecondFormat,
    initialHiddenSeries,
    initialSelection,
    logiTableRows as logiRows,
    neutPressureFor,
    ogDescription,
    ogTitle,
    peakDps as peakOf,
    percentFormat,
    pilotEvents as eventsOf,
    pilotTableRows,
    qualitySegments,
    selectionLabel as labelOf,
    tankSupportRatio as supportOf,
    urlQuery,
    weaponItems,
    wholePercentFormat,
    type EngagementDirection,
    type EngagementType,
  } from '#lib/analysis.ts';
  import AnimatedNumber from '#lib/components/animated-number.svelte';
  import DamageBarChart from '#lib/components/charts/damage-bar-chart.svelte';
  import DamageDoughnutChart from '#lib/components/charts/damage-doughnut-chart.svelte';
  import DpsLineChart from '#lib/components/charts/dps-line-chart.svelte';
  import QualitySpectrum from '#lib/components/charts/quality-spectrum.svelte';
  import DataTable, { type Column } from '#lib/components/data-table.svelte';
  import EventTypeIcon from '#lib/components/event-type-icon.svelte';
  import PageHead from '#lib/components/page-head.svelte';
  import PilotAvatar from '#lib/components/pilot-avatar.svelte';
  import ShipIcon from '#lib/components/ship-icon.svelte';
  import StatPanel from '#lib/components/stat-panel.svelte';
  import StatRow from '#lib/components/stat-row.svelte';
  import ZkillLink from '#lib/components/zkill-link.svelte';
  import { relativeTimeFromEve } from '#lib/dates.ts';
  import { analysisHead } from '#lib/seo.ts';
  import type { DateTimeRange, SeriesKey } from '#lib/types.ts';

  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  let analysis = $derived(data.analysis);
  let pilotIds = $derived(data.pilotIds);
  let shipTypeIds = $derived(data.shipTypeIds);

  // svelte-ignore state_referenced_locally
  let selection = $state<DateTimeRange | null>(initialSelection(data.filters));
  // svelte-ignore state_referenced_locally
  let hiddenSeries = $state<Set<SeriesKey>>(initialHiddenSeries(data.filters));
  // svelte-ignore state_referenced_locally
  let selectedPilot = $state<string | null>(data.filters.pilot);

  function toggleSeries(key: SeriesKey) {
    const next = new Set(hiddenSeries);

    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }

    hiddenSeries = next;
  }

  let regenerating = $state(false);
  let regenerateError = $state<string | null>(null);

  async function regenerateAnalysis() {
    regenerating = true;
    regenerateError = null;
    try {
      const response = await fetch(`/logs/${data.uuid}/regenerate`, {
        method: 'POST',
        headers: { Accept: 'application/json' },
      });
      if (response.ok) {
        await invalidateAll();
      } else {
        const answer = await response.json().catch(() => ({}));
        regenerateError = answer.message ?? 'Server Error';
      }
    } finally {
      regenerating = false;
    }
  }

  let dpsOverTime = $derived(bucketDps(analysis.events));
  let filteredEvents = $derived(filterEvents(analysis.events, selection));
  let summary = $derived(buildSummary(filteredEvents));
  let details = $derived(buildDetails(filteredEvents));
  let peakDps = $derived(peakOf(filteredEvents));
  let tankSupportRatio = $derived(supportOf(summary));
  let neutPressureIn = $derived(neutPressureFor(summary, details.neutIn));
  let neutPressureOut = $derived(neutPressureFor(summary, details.neutOut));
  let qualitySegmentsOut = $derived(qualitySegments(details.qualityOut));
  let qualitySegmentsIn = $derived(qualitySegments(details.qualityIn));
  let damageByTarget = $derived(buildDamageByEntity(filteredEvents, 'outgoing'));
  let damageByWeapon = $derived(buildWeaponBreakdown(filteredEvents, 'outgoing'));
  let incomingBySource = $derived(buildDamageByEntity(filteredEvents, 'incoming'));
  let incomingByWeapon = $derived(buildWeaponBreakdown(filteredEvents, 'incoming'));
  let outgoingTableRows = $derived(pilotTableRows(damageByTarget, summary));
  let incomingTableRows = $derived(pilotTableRows(incomingBySource, summary));
  let logiTableRows = $derived(logiRows(filteredEvents));
  let damageByTargetItems = $derived(entityItems(damageByTarget));
  let damageByWeaponItems = $derived(weaponItems(damageByWeapon));
  let incomingBySourceItems = $derived(entityItems(incomingBySource));
  let incomingByWeaponItems = $derived(weaponItems(incomingByWeapon));
  let selectionLabel = $derived(labelOf(selection));
  let relativeStart = $derived(relativeTimeFromEve(analysis.sessionStarted));

  let head = $derived(
    analysisHead(data.name, ogTitle(analysis.listener), ogDescription(summary), page.url),
  );

  const pilotColumns: Column[] = [
    { key: 'name', label: 'Pilot' },
    { key: 'ship', label: 'Ship' },
    { key: 'damage', label: 'Damage', align: 'right' },
    { key: 'dps', label: 'DPS', align: 'right' },
    { key: 'hits', label: 'Hits', align: 'right' },
    { key: 'misses', label: 'Misses', align: 'right' },
    { key: 'hitRate', label: 'Hit %', align: 'right' },
  ];

  const logiColumns: Column[] = [
    { key: 'name', label: 'Pilot' },
    { key: 'direction', label: 'Direction' },
    { key: 'ship', label: 'Ship' },
    { key: 'module', label: 'Module' },
    { key: 'amount', label: 'HP Repaired', align: 'right' },
    { key: 'count', label: 'Cycles', align: 'right' },
  ];

  let activeTab = $state<'outgoing' | 'incoming' | 'logistics'>('outgoing');

  // Pilot engagement drilldown
  function selectPilot(name: string) {
    selectedPilot = selectedPilot === name ? null : name;
  }

  let engagementDialog = $state<HTMLDialogElement | null>(null);

  $effect(() => {
    const pilot = selectedPilot;
    const dialog = engagementDialog;

    if (!dialog) {
      return;
    }

    if (pilot && !dialog.open) {
      dialog.showModal();
    } else if (!pilot && dialog.open) {
      dialog.close();
    }
  });

  function onEngagementDialogClick(event: MouseEvent) {
    if (event.target === engagementDialog) {
      engagementDialog?.close();
    }
  }

  const engagementTypeFilter = new SvelteSet<EngagementType>(ENGAGEMENT_TYPES);
  let engagementDirection = $state<EngagementDirection>('both');

  function toggleEngagementType(type: EngagementType) {
    if (engagementTypeFilter.has(type)) {
      engagementTypeFilter.delete(type);
    } else {
      engagementTypeFilter.add(type);
    }
  }

  let pilotEvents = $derived(eventsOf(filteredEvents, selectedPilot));
  let engagement = $derived(engagementOf(pilotEvents));
  let engagementLog = $derived(
    logOf(pilotEvents, new Set(engagementTypeFilter), engagementDirection),
  );

  // The URL keeps the selection, the hidden series and the open pilot,
  // so a link shares the view. It follows changes, not the first render.
  let mounted = false;

  $effect(() => {
    const query = urlQuery(
      selection,
      hiddenSeries,
      selectedPilot,
      new URLSearchParams(window.location.search),
    );

    if (!mounted || query === null) {
      return;
    }

    const search = new URLSearchParams(query).toString();
    replaceState(`${window.location.pathname}${search ? `?${search}` : ''}`, page.state);
  });

  onMount(() => {
    mounted = true;
  });
</script>

<PageHead {head} />

{#snippet pilotName(value: string)}
  <span class="flex items-center gap-2">
    {#if pilotIds[value] == null && shipTypeIds[value] != null}
      <ShipIcon name={value} typeId={shipTypeIds[value]} />
    {:else}
      <PilotAvatar name={value} characterId={pilotIds[value] ?? null} />
    {/if}
    <span>{value}</span>
    {#if pilotIds[value] != null}
      <ZkillLink characterId={pilotIds[value]} name={value} />
    {/if}
  </span>
{/snippet}

{#snippet shipName(value: string)}
  <span class="flex items-center gap-2">
    <ShipIcon name={value} typeId={shipTypeIds[value] ?? null} />
    <span>{value}</span>
  </span>
{/snippet}

{#snippet pilotCell({ key, value }: { key: string; value: string })}
  {#if key === 'name'}
    {@render pilotName(value)}
  {:else if key === 'ship'}
    {@render shipName(value)}
  {:else}
    {value}
  {/if}
{/snippet}

<div class="min-h-screen bg-zinc-950 text-zinc-300">
  <!-- Scanline overlay -->
  <div
    class="pointer-events-none fixed inset-0 opacity-[0.015]"
    style="
      background: repeating-linear-gradient(
        0deg,
        transparent,
        transparent 2px,
        rgba(161, 161, 170, 0.3) 2px,
        rgba(161, 161, 170, 0.3) 4px
      );
    "
  ></div>

  <div class="relative mx-auto max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
    <!-- After-action report header -->
    <header class="mb-6 border-b border-zinc-800 pb-6">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <!-- Crosshair icon -->
          <svg
            class="h-4 w-4 text-amber-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            stroke-width="1.5"
          >
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="2" x2="12" y2="6" />
            <line x1="12" y1="18" x2="12" y2="22" />
            <line x1="2" y1="12" x2="6" y2="12" />
            <line x1="18" y1="12" x2="22" y2="12" />
            <circle cx="12" cy="12" r="3" />
          </svg>
          <p class="font-mono text-xs tracking-widest text-zinc-400 uppercase">
            Combat Log Analyzer // After-Action Report
          </p>
        </div>
        <div class="flex items-center gap-4">
          <a
            href="https://github.com/eve-telescope/loganalyzer"
            target="_blank"
            rel="noopener noreferrer"
            title="View source on GitHub"
            class="text-zinc-400 transition-colors hover:text-amber-300"
          >
            <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 24 24">
              <path
                fill-rule="evenodd"
                clip-rule="evenodd"
                d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0 0 22 12.017C22 6.484 17.522 2 12 2Z"
              />
            </svg>
          </a>
          {#if data.rawLogAvailable && data.uuid}
            <a
              href="/logs/{data.uuid}/download"
              data-sveltekit-reload
              class="flex items-center gap-1.5 font-mono text-xs tracking-wider text-zinc-400 uppercase transition-colors hover:text-amber-300"
            >
              <!-- Download icon -->
              <svg
                class="h-3.5 w-3.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                stroke-width="1.5"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3"
                />
              </svg>
              Raw Log
            </a>
            <button
              type="button"
              disabled={regenerating}
              class="flex items-center gap-1.5 font-mono text-xs tracking-wider text-zinc-400 uppercase transition-colors hover:text-amber-300 disabled:cursor-wait disabled:text-zinc-600"
              onclick={regenerateAnalysis}
            >
              <svg
                class="h-3.5 w-3.5"
                class:animate-spin={regenerating}
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                stroke-width="1.5"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M16.023 9.348h4.992V4.356m-.97 5.52A8.25 8.25 0 106.27 18.45M3.985 14.652H8.98v4.992"
                />
              </svg>
              {regenerating ? 'Regenerating...' : 'Regenerate'}
            </button>
          {:else}
            <span
              class="font-mono text-xs tracking-wider text-zinc-600 uppercase"
              title="This log was uploaded before raw logs were kept, so it cannot be downloaded or regenerated."
              data-testid="raw-log-unavailable"
            >
              No raw log
            </span>
          {/if}
          <a
            href="/"
            class="flex items-center gap-1.5 font-mono text-xs tracking-wider text-zinc-400 uppercase transition-colors hover:text-amber-300"
          >
            <!-- Upload icon -->
            <svg
              class="h-3.5 w-3.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              stroke-width="1.5"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5"
              />
            </svg>
            New Log
          </a>
        </div>
      </div>

      {#if regenerateError}
        <p class="mt-3 font-mono text-xs text-red-400" role="alert">{regenerateError}</p>
      {/if}

      <h1 class="mt-5 font-mono text-2xl font-bold tracking-tight text-zinc-100 sm:text-3xl">
        {analysis.listener}
      </h1>
      <p class="mt-1.5 font-mono text-xs tracking-wider text-zinc-400 uppercase">
        {analysis.sessionStarted}
        {#if relativeStart}
          ({relativeStart})
        {/if}
        //
        {formatDuration(summary.combatDurationSeconds)}
        {selection ? 'selected' : ''} //
        <AnimatedNumber value={filteredEvents.length} /> events //
        <AnimatedNumber value={details.pilotsInvolved} />
        pilots
      </p>
    </header>

    <!-- Filter indicator -->
    {#if selection}
      <div
        class="mb-4 flex items-center gap-2 border-l-2 border-amber-400 bg-amber-950/20 px-3 py-2 font-mono text-xs tracking-wider text-amber-300 uppercase"
      >
        <svg
          class="h-3 w-3 shrink-0"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          stroke-width="2"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M12 3v17.25m0 0c-1.472 0-2.882.265-4.185.75M12 20.25c1.472 0 2.882.265 4.185.75M18.75 4.97A48.416 48.416 0 0012 4.5c-2.291 0-4.545.16-6.75.47m13.5 0c1.01.143 2.01.317 3 .52m-3-.52l2.62 10.726c.122.499-.106 1.028-.589 1.202a5.988 5.988 0 01-2.031.352 5.988 5.988 0 01-2.031-.352c-.483-.174-.711-.703-.59-1.202L18.75 4.97zm-16.5.52c.99-.203 1.99-.377 3-.52m0 0l2.62 10.726c.122.499-.106 1.028-.589 1.202a5.989 5.989 0 01-2.031.352 5.989 5.989 0 01-2.031-.352c-.483-.174-.711-.703-.59-1.202L5.25 4.97z"
          />
        </svg>
        Range: {selectionLabel}
      </div>
    {/if}

    <!-- Stat panels -->
    <section class="mb-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      <StatPanel title="Offense" accent="cyan">
        <p class="font-mono text-2xl font-bold text-cyan-400 tabular-nums">
          <AnimatedNumber value={summary.totalDamageDealt} />
        </p>
        <p class="mb-3 text-xs text-zinc-400">damage dealt</p>
        <dl class="space-y-1.5">
          <StatRow label="DPS avg" value={Math.round(summary.dpsDealt)} />
          <StatRow label="DPS peak (10s)" value={peakDps.dealt} />
          <StatRow
            label="Accuracy"
            value={accuracyValue(summary.totalOutgoingHits, summary.totalOutgoingMisses)}
            format={percentFormat}
          />
          <StatRow
            label="Biggest hit"
            value={details.biggestHitOut?.damage ?? '—'}
            hint={details.biggestHitOut
              ? `${details.biggestHitOut.weapon} → ${details.biggestHitOut.pilot}`
              : undefined}
          />
          <StatRow label="Targets engaged" value={details.uniqueTargets} />
        </dl>
      </StatPanel>

      <StatPanel title="Defense" accent="red">
        <p class="font-mono text-2xl font-bold text-red-400 tabular-nums">
          <AnimatedNumber value={summary.totalDamageReceived} />
        </p>
        <p class="mb-3 text-xs text-zinc-400">damage received</p>
        <dl class="space-y-1.5">
          <StatRow label="DPS avg" value={Math.round(summary.dpsReceived)} />
          <StatRow label="DPS peak (10s)" value={peakDps.received} />
          <StatRow
            label="Enemy accuracy"
            value={accuracyValue(summary.totalIncomingHits, summary.totalIncomingMisses)}
            format={percentFormat}
          />
          <StatRow
            label="Hardest hit taken"
            value={details.biggestHitIn?.damage ?? '—'}
            hint={details.biggestHitIn
              ? `${details.biggestHitIn.weapon} ← ${details.biggestHitIn.pilot}`
              : undefined}
          />
          <StatRow label="Unique attackers" value={details.uniqueAttackers} />
        </dl>
      </StatPanel>

      <StatPanel title="Logistics" accent="green">
        <p class="font-mono text-2xl font-bold text-emerald-400 tabular-nums">
          <AnimatedNumber value={summary.totalLogiReceived} />
        </p>
        <p class="mb-3 text-xs text-zinc-400">hp repaired onto you</p>
        <dl class="space-y-1.5">
          <StatRow label="Rep cycles received" value={details.logiInCycles} />
          <StatRow
            label="Tank support"
            value={tankSupportRatio}
            format={wholePercentFormat}
            hint="HP repaired onto you per HP of damage taken"
          />
          <StatRow label="HP repaired by you" value={summary.totalLogiDealt} />
          <StatRow label="Rep cycles dealt" value={details.logiOutCycles} />
        </dl>
      </StatPanel>

      <StatPanel title="Energy Warfare" accent="purple">
        <p class="font-mono text-2xl font-bold text-purple-400 tabular-nums">
          <AnimatedNumber value={details.neutIn} />
          <span class="text-sm font-medium text-purple-300/80"> GJ </span>
        </p>
        <p class="mb-3 text-xs text-zinc-400">drained from you</p>
        <dl class="space-y-1.5">
          <StatRow label="Pressure on you" value={neutPressureIn} format={gjPerSecondFormat} />
          <StatRow label="Neut events in" value={details.neutInEvents} />
          <StatRow label="Drained by you" value={details.neutOut} format={gjFormat} />
          <StatRow label="Pressure by you" value={neutPressureOut} format={gjPerSecondFormat} />
          <StatRow label="Neut events out" value={details.neutOutEvents} />
        </dl>
      </StatPanel>
    </section>

    <!-- Hit quality spectrums -->
    <section class="mb-6 grid gap-4 lg:grid-cols-2">
      <StatPanel title="Outgoing Hit Quality" accent="cyan">
        <QualitySpectrum segments={qualitySegmentsOut} hue="cyan" />
      </StatPanel>
      <StatPanel title="Incoming Hit Quality" accent="red">
        <QualitySpectrum segments={qualitySegmentsIn} hue="red" />
      </StatPanel>
    </section>

    <!-- DPS Timeline -->
    <section class="mb-6">
      <div class="mb-3 flex items-center gap-2">
        <!-- Chart icon -->
        <svg
          class="h-4 w-4 text-zinc-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          stroke-width="1.5"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 013 19.875v-6.75zM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V8.625zM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V4.125z"
          />
        </svg>
        <h2 class="font-mono text-sm font-medium tracking-widest text-zinc-300 uppercase">
          Timeline
        </h2>
      </div>
      <StatPanel>
        <DpsLineChart
          data={dpsOverTime}
          {selection}
          {hiddenSeries}
          onSelection={(range) => (selection = range)}
          onToggleSeries={toggleSeries}
        />
      </StatPanel>
    </section>

    <!-- Damage Breakdowns -->
    <section class="mb-6">
      <div class="mb-3 flex items-center gap-2">
        <!-- Bolt icon -->
        <svg
          class="h-4 w-4 text-zinc-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          stroke-width="1.5"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z"
          />
        </svg>
        <h2 class="font-mono text-sm font-medium tracking-widest text-zinc-300 uppercase">
          Damage Breakdown
        </h2>
      </div>
      <div class="grid gap-4 lg:grid-cols-2">
        <StatPanel title="Outgoing — By Target" accent="cyan">
          <DamageBarChart items={damageByTargetItems} />
        </StatPanel>
        <StatPanel title="Outgoing — By Weapon" accent="cyan">
          <DamageDoughnutChart items={damageByWeaponItems} />
        </StatPanel>
        <StatPanel title="Incoming — By Source" accent="red">
          <DamageBarChart items={incomingBySourceItems} color="#ef4444" />
        </StatPanel>
        <StatPanel title="Incoming — By Weapon" accent="red">
          <DamageDoughnutChart items={incomingByWeaponItems} />
        </StatPanel>
      </div>
    </section>

    <!-- Detailed Tables -->
    <section>
      <div class="mb-3 flex items-center gap-2">
        <!-- Table icon -->
        <svg
          class="h-4 w-4 text-zinc-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          stroke-width="1.5"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M3.375 19.5h17.25m-17.25 0a1.125 1.125 0 01-1.125-1.125M3.375 19.5h7.5c.621 0 1.125-.504 1.125-1.125m-9.75 0V5.625m0 12.75v-1.5c0-.621.504-1.125 1.125-1.125m18.375 2.625V5.625m0 12.75c0 .621-.504 1.125-1.125 1.125m1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125m0 3.75h-7.5A1.125 1.125 0 0112 18.375m9.75-12.75c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125m19.5 0v1.5c0 .621-.504 1.125-1.125 1.125M2.25 5.625v1.5c0 .621.504 1.125 1.125 1.125m0 0h17.25m-17.25 0h7.5c.621 0 1.125.504 1.125 1.125M3.375 8.25c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125m17.25-3.75h-7.5c-.621 0-1.125.504-1.125 1.125m8.625-1.125c.621 0 1.125.504 1.125 1.125v1.5c0 .621-.504 1.125-1.125 1.125m-17.25 0h7.5m-7.5 0c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125M12 10.875v-1.5m0 1.5c0 .621-.504 1.125-1.125 1.125M12 10.875c0 .621.504 1.125 1.125 1.125m-2.25 0c.621 0 1.125.504 1.125 1.125M12 12h7.5m-7.5 0c-.621 0-1.125.504-1.125 1.125M21.375 12c.621 0 1.125.504 1.125 1.125v1.5c0 .621-.504 1.125-1.125 1.125M12 17.25v-5.25m0 5.25c0 .621.504 1.125 1.125 1.125h2.25c.621 0 1.125-.504 1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125H12m0 3.75c0 .621-.504 1.125-1.125 1.125H8.625A1.125 1.125 0 017.5 17.25v-1.5c0-.621.504-1.125 1.125-1.125H12"
          />
        </svg>
        <h2 class="font-mono text-sm font-medium tracking-widest text-zinc-300 uppercase">
          Pilot Details
        </h2>
      </div>
      <StatPanel>
        <div class="mb-4 flex gap-px">
          {#each ['outgoing', 'incoming', 'logistics'] as const as tab (tab)}
            <button
              class="border-b-2 px-4 py-2 font-mono text-xs tracking-widest uppercase transition-colors {activeTab ===
              tab
                ? 'border-amber-400 text-amber-300'
                : 'border-transparent text-zinc-400 hover:text-zinc-200'}"
              onclick={() => (activeTab = tab)}
            >
              {tab === 'outgoing' ? 'Outgoing' : tab === 'incoming' ? 'Incoming' : 'Logistics'}
            </button>
          {/each}
        </div>

        {#if activeTab === 'outgoing'}
          <DataTable
            columns={pilotColumns}
            rows={outgoingTableRows}
            emptyText="No outgoing damage events"
            clickable
            selectedKey="name"
            selectedValue={selectedPilot}
            onRowClick={(row) => selectPilot(String(row.name))}
            cell={pilotCell}
          />
        {:else if activeTab === 'incoming'}
          <DataTable
            columns={pilotColumns}
            rows={incomingTableRows}
            emptyText="No incoming damage events"
            clickable
            selectedKey="name"
            selectedValue={selectedPilot}
            onRowClick={(row) => selectPilot(String(row.name))}
            cell={pilotCell}
          />
        {:else}
          <DataTable
            columns={logiColumns}
            rows={logiTableRows}
            emptyText="No logistics events"
            clickable
            selectedKey="name"
            selectedValue={selectedPilot}
            onRowClick={(row) => selectPilot(String(row.name))}
            cell={pilotCell}
          />
        {/if}
      </StatPanel>
    </section>

    <!-- Pilot engagement drilldown -->
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
    <dialog
      bind:this={engagementDialog}
      class="m-auto w-full max-w-6xl bg-transparent p-4 backdrop:bg-zinc-950/60 backdrop:backdrop-blur-[2px]"
      onclose={() => (selectedPilot = null)}
      onclick={onEngagementDialogClick}
    >
      {#if selectedPilot}
        <div class="h-[90vh] bg-zinc-950">
          <StatPanel
            title="Engagement — {selectedPilot}"
            class="flex h-full flex-col overflow-hidden"
          >
            <div class="flex items-start justify-between gap-4">
              <div class="flex items-center gap-3">
                {#if pilotIds[selectedPilot] == null && shipTypeIds[selectedPilot] != null}
                  <ShipIcon name={selectedPilot} typeId={shipTypeIds[selectedPilot]} />
                {:else}
                  <PilotAvatar
                    name={selectedPilot}
                    characterId={pilotIds[selectedPilot] ?? null}
                    size="md"
                  />
                {/if}
                <div>
                  <p class="flex items-center gap-2 font-mono text-lg font-semibold text-zinc-100">
                    {selectedPilot}
                    {#if pilotIds[selectedPilot] != null}
                      <ZkillLink characterId={pilotIds[selectedPilot]} name={selectedPilot} />
                    {/if}
                  </p>
                  {#if engagement.ship}
                    <p class="flex items-center gap-1.5 text-xs text-zinc-400">
                      <ShipIcon
                        name={engagement.ship}
                        typeId={shipTypeIds[engagement.ship] ?? null}
                      />
                      {engagement.ship}
                    </p>
                  {/if}
                </div>
              </div>
              <button
                class="font-mono text-xs tracking-wider text-zinc-400 uppercase transition-colors hover:text-amber-300"
                onclick={() => (selectedPilot = null)}
              >
                Close ✕
              </button>
            </div>

            <div class="mt-4 grid gap-4 sm:grid-cols-2">
              <div>
                <p class="mb-2 font-mono text-xs tracking-widest text-cyan-400 uppercase">
                  You → {selectedPilot}
                </p>
                <dl class="space-y-1.5">
                  <StatRow label="Damage dealt" value={engagement.dealt} />
                  <StatRow
                    label="Hits / misses"
                    value={`${engagement.hitsOn} / ${engagement.missesOn}`}
                  />
                  <StatRow
                    label="Accuracy"
                    value={accuracyValue(engagement.hitsOn, engagement.missesOn)}
                    format={percentFormat}
                  />
                  {#if engagement.logiOut > 0}
                    <StatRow label="HP repaired onto them" value={engagement.logiOut} />
                  {/if}
                  {#if engagement.neutOut > 0}
                    <StatRow
                      label="Energy drained from them"
                      value={engagement.neutOut}
                      format={gjFormat}
                    />
                  {/if}
                </dl>
              </div>
              <div>
                <p class="mb-2 font-mono text-xs tracking-widest text-red-400 uppercase">
                  {selectedPilot} → You
                </p>
                <dl class="space-y-1.5">
                  <StatRow label="Damage received" value={engagement.received} />
                  <StatRow
                    label="Hits / misses"
                    value={`${engagement.hitsBy} / ${engagement.missesBy}`}
                  />
                  <StatRow
                    label="Accuracy"
                    value={accuracyValue(engagement.hitsBy, engagement.missesBy)}
                    format={percentFormat}
                  />
                  {#if engagement.logiIn > 0}
                    <StatRow label="HP repaired onto you" value={engagement.logiIn} />
                  {/if}
                  {#if engagement.neutIn > 0}
                    <StatRow
                      label="Energy drained from you"
                      value={engagement.neutIn}
                      format={gjFormat}
                    />
                  {/if}
                </dl>
              </div>
            </div>

            <div class="mt-5 flex flex-wrap items-center gap-2">
              {#each ['both', 'outgoing', 'incoming'] as const as direction (direction)}
                <button
                  class="border px-2.5 py-1 font-mono text-xs tracking-wider uppercase transition-colors {engagementDirection ===
                  direction
                    ? 'border-amber-400/60 bg-amber-400/10 text-amber-300'
                    : 'border-zinc-700 text-zinc-400 hover:text-zinc-200'}"
                  onclick={() => (engagementDirection = direction)}
                >
                  {direction}
                </button>
              {/each}
              <span class="mx-1 h-4 w-px bg-zinc-700"></span>
              {#each ENGAGEMENT_TYPES as type (type)}
                <button
                  class="flex items-center gap-1.5 border px-2.5 py-1 font-mono text-xs tracking-wider uppercase transition-colors {engagementTypeFilter.has(
                    type,
                  )
                    ? 'border-amber-400/60 bg-amber-400/10 text-amber-300'
                    : 'border-zinc-700 text-zinc-400 hover:text-zinc-200'}"
                  onclick={() => toggleEngagementType(type)}
                >
                  <EventTypeIcon {type} />
                  {ENGAGEMENT_TYPE_LABELS[type]}
                </button>
              {/each}
            </div>

            <ul class="mt-3 min-h-0 flex-1 divide-y divide-zinc-800/60 overflow-y-auto">
              {#each engagementLog as e, i (i)}
                <li class="flex items-center gap-3 py-1.5">
                  <span class="font-mono text-xs text-zinc-500 tabular-nums">
                    {e.time}
                  </span>
                  <span class={eventColorClass(e)}>
                    <EventTypeIcon type={e.type} />
                  </span>
                  <span class="font-mono text-xs {eventColorClass(e)}">
                    {e.direction === 'outgoing' ? '→' : '←'}
                  </span>
                  <span
                    class="w-20 shrink-0 text-right font-mono text-sm font-medium tabular-nums {eventColorClass(
                      e,
                    )}"
                  >
                    {formatNumber(e.damage)}{e.type === 'neutralization' ? ' GJ' : ''}
                  </span>
                  <span class="truncate text-xs text-zinc-300">
                    {e.weapon}
                  </span>
                  <span class="ml-auto shrink-0 font-mono text-[11px] text-zinc-500 uppercase">
                    {e.quality}
                  </span>
                </li>
              {/each}
              {#if engagementLog.length === 0}
                <li class="py-4 text-center text-sm text-zinc-400">No events match the filters</li>
              {/if}
            </ul>
          </StatPanel>
        </div>
      {/if}
    </dialog>
  </div>
</div>
