<?php

declare(strict_types=1);

namespace App\Models;

use Database\Factories\CombatLogFactory;
use Illuminate\Database\Eloquent\Attributes\Fillable;
use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;

#[Fillable(['uuid', 'listener', 'session_started', 'original_filename'])]
final class CombatLog extends Model
{
    /** @use HasFactory<CombatLogFactory> */
    use HasFactory;

    public function getRouteKeyName(): string
    {
        return 'uuid';
    }

    /**
     * @return HasMany<CombatEvent, $this>
     */
    public function events(): HasMany
    {
        return $this->hasMany(CombatEvent::class);
    }
}
