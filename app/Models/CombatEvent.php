<?php

declare(strict_types=1);

namespace App\Models;

use App\Enums\EventDirection;
use App\Enums\EventType;
use Illuminate\Database\Eloquent\Attributes\Fillable;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

#[Fillable(['combat_log_id', 'timestamp', 'damage', 'direction', 'player_name', 'corporation', 'ship_name', 'weapon', 'quality', 'type'])]
final class CombatEvent extends Model
{
    public $timestamps = false;

    /**
     * @return BelongsTo<CombatLog, $this>
     */
    public function combatLog(): BelongsTo
    {
        return $this->belongsTo(CombatLog::class);
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'direction' => EventDirection::class,
            'type' => EventType::class,
        ];
    }
}
