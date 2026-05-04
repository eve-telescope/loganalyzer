<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Http\Requests\StoreCombatLogRequest;
use App\Models\CombatEvent;
use App\Models\CombatLog;
use App\Services\CombatLogParser;
use Illuminate\Http\RedirectResponse;
use Illuminate\Support\Str;
use Inertia\Inertia;
use Inertia\Response;

final class CombatLogController extends Controller
{
    public function create(): Response
    {
        return Inertia::render('Upload', [
            'maxUploadSizeMb' => (int) config('loganalyzer.upload.max_size_mb'),
        ]);
    }

    public function store(StoreCombatLogRequest $request, CombatLogParser $parser): RedirectResponse
    {
        $file = $request->file('log_file');
        $parsed = $parser->parse($file->get());

        $combatLog = CombatLog::create([
            'uuid' => Str::uuid()->toString(),
            'listener' => $parsed['listener'],
            'session_started' => $parsed['sessionStarted'],
            'original_filename' => $file->getClientOriginalName(),
        ]);

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

    public function show(CombatLog $combatLog): Response
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

        return Inertia::render('Analysis', [
            'analysis' => [
                'listener' => $combatLog->listener,
                'sessionStarted' => $combatLog->session_started,
                'events' => $mappedEvents,
            ],
            'uuid' => $combatLog->uuid,
        ]);
    }
}
