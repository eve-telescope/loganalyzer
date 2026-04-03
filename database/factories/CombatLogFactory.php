<?php

declare(strict_types=1);

namespace Database\Factories;

use App\Models\CombatLog;
use Illuminate\Database\Eloquent\Factories\Factory;
use Illuminate\Support\Str;

/**
 * @extends Factory<CombatLog>
 */
final class CombatLogFactory extends Factory
{
    /**
     * @return array<string, mixed>
     */
    public function definition(): array
    {
        return [
            'uuid' => Str::uuid()->toString(),
            'listener' => fake()->name(),
            'session_started' => now()->format('Y.m.d H:i:s'),
            'original_filename' => 'combat_log.txt',
        ];
    }
}
