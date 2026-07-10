<?php

declare(strict_types=1);

namespace Database\Factories;

use App\Enums\EveEntityKind;
use App\Models\EveEntity;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<EveEntity>
 */
final class EveEntityFactory extends Factory
{
    protected $model = EveEntity::class;

    public function definition(): array
    {
        return [
            'kind' => EveEntityKind::Character,
            'name' => $this->faker->unique()->name(),
            'eve_id' => $this->faker->numberBetween(90_000_000, 98_000_000),
        ];
    }

    public function inventoryType(): self
    {
        return $this->state([
            'kind' => EveEntityKind::InventoryType,
            'eve_id' => $this->faker->numberBetween(500, 90_000),
        ]);
    }
}
