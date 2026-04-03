<?php

declare(strict_types=1);

namespace App\Data;

use App\Enums\EventDirection;
use App\Enums\EventType;

final class CombatEvent
{
    public function __construct(
        public string $timestamp,
        public int $damage,
        public EventDirection $direction,
        public string $playerName,
        public ?string $corporation,
        public ?string $shipName,
        public string $weapon,
        public string $quality,
        public EventType $type = EventType::Damage,
    ) {}
}
