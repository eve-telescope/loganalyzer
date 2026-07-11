<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Http\Requests\StoreCombatLogRequest;
use App\Models\CombatEvent;
use App\Models\CombatLog;
use App\Models\EveEntity;
use App\Services\CombatLogParser;
use App\Services\EveEntityResolver;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Storage;
use Illuminate\Support\Str;
use Inertia\Inertia;
use Inertia\Response;
use Symfony\Component\HttpFoundation\StreamedResponse;

final class CombatLogController extends Controller
{
    public function create(): Response
    {
        return Inertia::render('Upload', [
            'maxUploadSizeMb' => (int) config('loganalyzer.upload.max_size_mb'),
        ]);
    }

    public function store(StoreCombatLogRequest $request, CombatLogParser $parser, EveEntityResolver $resolver): RedirectResponse
    {
        $file = $request->file('log_file');
        $contents = $file->get();
        $parsed = $parser->parse($contents);

        $resolver->resolve(
            characterNames: array_map(fn (\App\Data\CombatEvent $e): string => $e->playerName, $parsed['events']),
            typeNames: array_values(array_filter(array_map(fn (\App\Data\CombatEvent $e): ?string => $e->shipName, $parsed['events']))),
        );

        $combatLog = CombatLog::create([
            'uuid' => Str::uuid()->toString(),
            'listener' => $parsed['listener'],
            'session_started' => $parsed['sessionStarted'],
            'original_filename' => $file->getClientOriginalName(),
        ]);

        Storage::put($this->rawLogPath($combatLog), gzencode($contents, 9));

        $rows = array_map(fn (\App\Data\CombatEvent $event): array => [
            'combat_log_id' => $combatLog->id,
            'timestamp' => $event->timestamp,
            'damage' => $event->damage,
            'direction' => $event->direction->value,
            'player_name' => $event->playerName,
            'corporation' => $event->corporation,
            'ship_name' => $event->shipName,
            'weapon' => $event->weapon,
            'quality' => $event->quality,
            'type' => $event->type->value,
        ], $parsed['events']);

        foreach (array_chunk($rows, 500) as $chunk) {
            $combatLog->events()->insert($chunk);
        }

        return redirect()->route('combat-log.show', $combatLog);
    }

    public function show(CombatLog $combatLog, Request $request): Response
    {
        $events = $combatLog->events()
            ->select(['timestamp', 'damage', 'direction', 'player_name', 'corporation', 'ship_name', 'weapon', 'quality', 'type'])
            ->get();

        $mappedEvents = $events->map(fn (CombatEvent $e): array => [
            'timestamp' => $e->timestamp,
            'damage' => $e->damage,
            'direction' => $e->getRawOriginal('direction'),
            'playerName' => $e->player_name,
            'corporation' => $e->corporation,
            'shipName' => $e->ship_name,
            'weapon' => $e->weapon,
            'quality' => $e->quality,
            'type' => $e->getRawOriginal('type'),
        ]);

        $hide = (string) $request->query('hide', '');

        $names = $events->pluck('player_name')
            ->merge($events->pluck('ship_name')->filter())
            ->unique()
            ->values();

        $entities = EveEntity::query()
            ->whereIn('name', $names)
            ->whereNotNull('eve_id')
            ->get();

        return Inertia::render('Analysis', [
            'analysis' => [
                'listener' => $combatLog->listener,
                'sessionStarted' => $combatLog->session_started,
                'events' => $mappedEvents,
            ],
            'uuid' => $combatLog->uuid,
            'rawLogAvailable' => Storage::exists($this->rawLogPath($combatLog)),
            'pilotIds' => $entities
                ->where('kind', \App\Enums\EveEntityKind::Character)
                ->pluck('eve_id', 'name'),
            'shipTypeIds' => $entities
                ->where('kind', \App\Enums\EveEntityKind::InventoryType)
                ->pluck('eve_id', 'name'),
            'filters' => [
                'from' => $request->query('from'),
                'to' => $request->query('to'),
                'pilot' => $request->query('pilot'),
                'hide' => $hide === ''
                    ? []
                    : array_values(array_filter(array_map(trim(...), explode(',', $hide)))),
            ],
        ]);
    }

    public function download(CombatLog $combatLog): StreamedResponse
    {
        $path = $this->rawLogPath($combatLog);

        abort_unless(Storage::exists($path), 404);

        return response()->streamDownload(
            function () use ($path): void {
                echo gzdecode(Storage::get($path));
            },
            $combatLog->original_filename ?: 'combat-log.txt',
            ['Content-Type' => 'text/plain; charset=utf-8'],
        );
    }

    private function rawLogPath(CombatLog $combatLog): string
    {
        return "combat-logs/{$combatLog->uuid}.txt.gz";
    }
}
