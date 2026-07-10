<?php

declare(strict_types=1);

namespace App\Services;

use App\Data\CombatEvent;
use App\Enums\EventDirection;
use App\Enums\EventType;

final class CombatLogParser
{
    /**
     * @return array{listener: string, sessionStarted: string, events: list<CombatEvent>}
     */
    public function parse(string $contents): array
    {
        return [
            ...$this->parseHeader($contents),
            'events' => $this->attributeDroneDamage($this->parseEvents($contents)),
        ];
    }

    /**
     * Outgoing damage to a drone logs the drone itself as the target
     * (e.g. "Hammerhead II[.BOP](Hammerhead II)"), so the pilot slot holds
     * a type name. The owner is inferred from the rest of the log: pilots
     * appear with the drone name as their weapon on other lines, narrowed
     * by the corp ticker the drone entry carries. Ambiguous drones (several
     * candidate owners) are left untouched.
     *
     * @param  list<CombatEvent>  $events
     * @return list<CombatEvent>
     */
    private function attributeDroneDamage(array $events): array
    {
        $weaponUsers = [];
        $corpPilots = [];

        foreach ($events as $event) {
            if ($event->shipName !== null && $event->playerName === $event->shipName) {
                continue;
            }

            $weaponUsers[$event->weapon][$event->playerName] = true;

            if ($event->corporation !== null) {
                $corpPilots[$event->corporation][$event->playerName] = true;
            }
        }

        return array_map(function (CombatEvent $event) use ($weaponUsers, $corpPilots): CombatEvent {
            $isDroneTarget = $event->type === EventType::Damage
                && $event->shipName !== null
                && $event->playerName === $event->shipName;

            if (! $isDroneTarget) {
                return $event;
            }

            $candidates = array_keys($weaponUsers[$event->playerName] ?? []);

            if ($event->corporation !== null && isset($corpPilots[$event->corporation])) {
                $corpNames = array_keys($corpPilots[$event->corporation]);
                $narrowed = array_values(array_intersect($candidates, $corpNames));

                if ($narrowed !== []) {
                    $candidates = $narrowed;
                } elseif ($candidates === [] && count($corpNames) === 1) {
                    $candidates = $corpNames;
                }
            }

            if (count($candidates) !== 1) {
                return $event;
            }

            return new CombatEvent(
                timestamp: $event->timestamp,
                damage: $event->damage,
                direction: $event->direction,
                playerName: $candidates[0],
                corporation: $event->corporation,
                shipName: $event->shipName,
                weapon: $event->weapon,
                quality: $event->quality,
                type: $event->type,
            );
        }, $events);
    }

    /**
     * @return array{listener: string, sessionStarted: string}
     */
    private function parseHeader(string $contents): array
    {
        $listener = 'Unknown';
        $sessionStarted = '';

        if (preg_match('/^\s*Listener:\s*(.+)$/mu', $contents, $match)) {
            $listener = $this->clean($match[1]);
        }

        if (preg_match('/^\s*Session Started:\s*(.+)$/mu', $contents, $match)) {
            $sessionStarted = $this->clean($match[1]);
        }

        return [
            'listener' => $listener,
            'sessionStarted' => $sessionStarted,
        ];
    }

    /**
     * @return list<CombatEvent>
     */
    private function parseEvents(string $contents): array
    {
        $events = [];

        preg_match_all(
            '/^\[ (\d{4}\.\d{2}\.\d{2} \d{2}:\d{2}:\d{2}) \] \(combat\) (.+)$/mu',
            $contents,
            $matches,
            PREG_SET_ORDER,
        );

        foreach ($matches as $match) {
            $event = $this->parseCombatLine($match[1], $match[2]);

            if ($event instanceof CombatEvent) {
                $events[] = $event;
            }
        }

        return $events;
    }

    private function parseCombatLine(string $timestamp, string $content): ?CombatEvent
    {
        // Logistics received
        if (preg_match('/<b>(\d+)<\/b>.*?remote (shield|armor|hull) (boosted|repaired) by.*?<b>([^<]+)<\/b>.*?<font size=10>([^<]+)<\/font><\/b>.*? - (.+?)(?:<\/font>)?$/u', $content, $match)) {
            return new CombatEvent(
                timestamp: $timestamp,
                damage: (int) $match[1],
                direction: EventDirection::Incoming,
                playerName: $this->clean($match[5]),
                corporation: null,
                shipName: $this->clean($match[4]),
                weapon: $this->clean($match[6]),
                quality: $this->clean($match[2]).' '.$this->clean($match[3]),
                type: EventType::Logistics,
            );
        }

        // Logistics received — plain target variant ("Pilot[CORP](Ship)"
        // wrapped in <color=0xffffffff>, like damage lines). Reported in #1:
        // these lines were silently dropped by the pattern above.
        if (preg_match('/<b>(\d+)<\/b>.*?remote (shield|armor|hull) (boosted|repaired) by.*?<b><color=0xffffffff>(.+?)<\/b>.*? - (.+?)(?:<\/font>)?$/u', $content, $match)) {
            $target = $this->parseTarget($this->clean($match[4]));

            return new CombatEvent(
                timestamp: $timestamp,
                damage: (int) $match[1],
                direction: EventDirection::Incoming,
                playerName: $target['name'],
                corporation: $target['corporation'],
                shipName: $target['ship'],
                weapon: $this->clean($match[5]),
                quality: $this->clean($match[2]).' '.$this->clean($match[3]),
                type: EventType::Logistics,
            );
        }

        // Logistics dealt
        if (preg_match('/<b>(\d+)<\/b>.*?remote (shield|armor|hull) (boosted|repaired) to.*?<b><color=0xffffffff>(.+?)<\/b>.*? - (.+)$/u', $content, $match)) {
            return new CombatEvent(
                timestamp: $timestamp,
                damage: (int) $match[1],
                direction: EventDirection::Outgoing,
                playerName: $this->clean($match[4]),
                corporation: null,
                shipName: null,
                weapon: $this->clean($match[5]),
                quality: $this->clean($match[2]).' '.$this->clean($match[3]),
                type: EventType::Logistics,
            );
        }

        // Energy neutralization / nosferatu. Direction depends on (verb, preposition):
        //   "neutralized {ship}"       → incoming neut (verified)
        //   "drained to {ship}"        → incoming nos (verified, negative GJ)
        //   "neutralized to {ship}"    → outgoing neut (best-effort, no fixture)
        //   "drained from {ship}"      → outgoing nos (best-effort, no fixture)
        if (preg_match('/<b>-?(\d+) GJ<\/b>.*?energy (neutralized|drained)(?: (to|from))? .*?<b>([^<]+)<\/b>.*? - (.+?)<\/font>/u', $content, $match)) {
            $verb = $match[2];
            $preposition = $match[3];

            $direction = match (true) {
                $preposition === 'from' => EventDirection::Outgoing,
                $verb === 'neutralized' && $preposition === 'to' => EventDirection::Outgoing,
                default => EventDirection::Incoming,
            };

            return new CombatEvent(
                timestamp: $timestamp,
                damage: (int) $match[1],
                direction: $direction,
                playerName: $this->clean($match[4]),
                corporation: null,
                shipName: $this->clean($match[4]),
                weapon: $this->clean($match[5]),
                quality: $verb === 'neutralized' ? 'Neutralized' : 'Drained',
                type: EventType::Neutralization,
            );
        }

        // Damage hit
        if (preg_match('/<b>(\d+)<\/b>.*?<font size=10>(to|from)<\/font>.*?<b><color=0xffffffff>(.+?)<\/b>.*? - (.+?) - (Hits|Penetrates|Glances Off|Grazes|Smashes|Wrecks)/u', $content, $match)) {
            $target = $this->parseTarget($match[3]);

            return new CombatEvent(
                timestamp: $timestamp,
                damage: (int) $match[1],
                direction: $match[2] === 'to' ? EventDirection::Outgoing : EventDirection::Incoming,
                playerName: $target['name'],
                corporation: $target['corporation'],
                shipName: $target['ship'],
                weapon: $this->clean($match[4]),
                quality: $match[5],
            );
        }

        // Incoming miss
        if (preg_match('/^(.+?) belonging to (.+?) misses you completely - (.+)$/u', $content, $match)) {
            return new CombatEvent(
                timestamp: $timestamp,
                damage: 0,
                direction: EventDirection::Incoming,
                playerName: $this->clean($match[2]),
                corporation: null,
                shipName: null,
                weapon: $this->clean($match[1]),
                quality: 'Misses',
            );
        }

        // Outgoing miss
        if (preg_match('/^Your (.+?) misses (.+?) completely - (.+)$/u', $content, $match)) {
            return new CombatEvent(
                timestamp: $timestamp,
                damage: 0,
                direction: EventDirection::Outgoing,
                playerName: $this->clean($match[2]),
                corporation: null,
                shipName: null,
                weapon: $this->clean($match[1]),
                quality: 'Misses',
            );
        }

        return null;
    }

    /**
     * Captured log fragments can retain markup (e.g. a trailing </font>)
     * depending on where a line's tag boundaries fall, so every captured
     * field is stripped centrally instead of per-pattern.
     */
    private function clean(string $value): string
    {
        return mb_trim(strip_tags($value));
    }

    /**
     * @return array{name: string, corporation: string|null, ship: string|null}
     */
    private function parseTarget(string $raw): array
    {
        if (preg_match('/^(.+?)\[([^\]]*)\]\(([^)]+)\)$/u', $raw, $match)) {
            return [
                'name' => $this->clean($match[1]),
                'corporation' => $this->clean($match[2]) ?: null,
                'ship' => $this->clean($match[3]),
            ];
        }

        return [
            'name' => $this->clean($raw),
            'corporation' => null,
            'ship' => null,
        ];
    }
}
