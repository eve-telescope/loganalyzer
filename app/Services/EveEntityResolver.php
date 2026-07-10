<?php

declare(strict_types=1);

namespace App\Services;

use App\Enums\EveEntityKind;
use App\Esi\ResolveUniverseIdsRequest;
use App\Models\EveEntity;
use NicolasKion\Esi\Connector;

final class EveEntityResolver
{
    private const int MAX_NAMES_PER_REQUEST = 500;

    /**
     * Resolves pilot and ship names to EVE character/type IDs, caching every
     * resolution (including misses) in the eve_entities table so each name
     * only ever hits ESI once. On ESI failure the unresolved names are left
     * uncached and the maps return null for them.
     *
     * @param  list<string>  $characterNames
     * @param  list<string>  $typeNames
     * @return array{characters: array<string, int|null>, types: array<string, int|null>}
     */
    public function resolve(array $characterNames, array $typeNames): array
    {
        $characterNames = array_values(array_unique(array_filter($characterNames)));
        $typeNames = array_values(array_unique(array_filter($typeNames)));

        $known = EveEntity::query()
            ->whereIn('name', [...$characterNames, ...$typeNames])
            ->get();

        $knownByKind = fn (EveEntityKind $kind): array => $known
            ->where('kind', $kind)
            ->pluck('eve_id', 'name')
            ->all();

        $knownCharacters = $knownByKind(EveEntityKind::Character);
        $knownTypes = $knownByKind(EveEntityKind::InventoryType);

        $missing = [
            ...array_diff($characterNames, array_keys($knownCharacters)),
            ...array_diff($typeNames, array_keys($knownTypes)),
        ];

        foreach (array_chunk(array_values(array_unique($missing)), self::MAX_NAMES_PER_REQUEST) as $chunk) {
            $result = (new Connector)->send(new ResolveUniverseIdsRequest($chunk));

            if ($result->failed()) {
                continue;
            }

            /** @var array<string, list<array{id: int, name: string}>> $data */
            $data = $result->data;

            $resolvedCharacters = array_column($data['characters'] ?? [], 'id', 'name');
            $resolvedTypes = array_column($data['inventory_types'] ?? [], 'id', 'name');

            $rows = [];

            foreach (array_intersect($characterNames, $chunk) as $name) {
                $rows[] = [
                    'kind' => EveEntityKind::Character->value,
                    'name' => $name,
                    'eve_id' => $resolvedCharacters[$name] ?? null,
                ];
            }

            foreach (array_intersect($typeNames, $chunk) as $name) {
                $rows[] = [
                    'kind' => EveEntityKind::InventoryType->value,
                    'name' => $name,
                    'eve_id' => $resolvedTypes[$name] ?? null,
                ];
            }

            EveEntity::query()->upsert($rows, ['kind', 'name'], ['eve_id']);

            $knownCharacters = [...$knownCharacters, ...array_intersect_key($resolvedCharacters, array_flip($characterNames))];
            $knownTypes = [...$knownTypes, ...array_intersect_key($resolvedTypes, array_flip($typeNames))];
        }

        return [
            'characters' => $this->buildMap($characterNames, $knownCharacters),
            'types' => $this->buildMap($typeNames, $knownTypes),
        ];
    }

    /**
     * @param  list<string>  $names
     * @param  array<string, int|null>  $resolved
     * @return array<string, int|null>
     */
    private function buildMap(array $names, array $resolved): array
    {
        $map = [];

        foreach ($names as $name) {
            $value = $resolved[$name] ?? null;
            $map[$name] = $value === null ? null : (int) $value;
        }

        return $map;
    }
}
