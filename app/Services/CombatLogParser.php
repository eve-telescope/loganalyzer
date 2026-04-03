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
            'events' => $this->parseEvents($contents),
        ];
    }

    /**
     * @return array{listener: string, sessionStarted: string}
     */
    private function parseHeader(string $contents): array
    {
        $listener = 'Unknown';
        $sessionStarted = '';

        if (preg_match('/^\s*Listener:\s*(.+)$/mu', $contents, $match)) {
            $listener = mb_trim($match[1]);
        }

        if (preg_match('/^\s*Session Started:\s*(.+)$/mu', $contents, $match)) {
            $sessionStarted = mb_trim($match[1]);
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
                playerName: mb_trim($match[5]),
                corporation: null,
                shipName: mb_trim($match[4]),
                weapon: mb_trim($match[6]),
                quality: mb_trim($match[2]).' '.mb_trim($match[3]),
                type: EventType::Logistics,
            );
        }

        // Logistics dealt
        if (preg_match('/<b>(\d+)<\/b>.*?remote (shield|armor|hull) (boosted|repaired) to.*?<b><color=0xffffffff>(.+?)<\/b>.*? - (.+)$/u', $content, $match)) {
            return new CombatEvent(
                timestamp: $timestamp,
                damage: (int) $match[1],
                direction: EventDirection::Outgoing,
                playerName: mb_trim($match[4]),
                corporation: null,
                shipName: null,
                weapon: mb_trim($match[5]),
                quality: mb_trim($match[2]).' '.mb_trim($match[3]),
                type: EventType::Logistics,
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
                weapon: mb_trim($match[4]),
                quality: $match[5],
            );
        }

        // Incoming miss
        if (preg_match('/^(.+?) belonging to (.+?) misses you completely - (.+)$/u', $content, $match)) {
            return new CombatEvent(
                timestamp: $timestamp,
                damage: 0,
                direction: EventDirection::Incoming,
                playerName: mb_trim($match[2]),
                corporation: null,
                shipName: null,
                weapon: mb_trim($match[1]),
                quality: 'Misses',
            );
        }

        // Outgoing miss
        if (preg_match('/^Your (.+?) misses (.+?) completely - (.+)$/u', $content, $match)) {
            return new CombatEvent(
                timestamp: $timestamp,
                damage: 0,
                direction: EventDirection::Outgoing,
                playerName: mb_trim($match[2]),
                corporation: null,
                shipName: null,
                weapon: mb_trim($match[1]),
                quality: 'Misses',
            );
        }

        return null;
    }

    /**
     * @return array{name: string, corporation: string|null, ship: string|null}
     */
    private function parseTarget(string $raw): array
    {
        if (preg_match('/^(.+?)\[([^\]]*)\]\(([^)]+)\)$/u', $raw, $match)) {
            return [
                'name' => mb_trim($match[1]),
                'corporation' => mb_trim($match[2]) ?: null,
                'ship' => mb_trim($match[3]),
            ];
        }

        return [
            'name' => mb_trim($raw),
            'corporation' => null,
            'ship' => null,
        ];
    }
}
