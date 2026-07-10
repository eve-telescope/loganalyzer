<?php

declare(strict_types=1);

namespace App\Services;

use App\Enums\EveEntityKind;
use App\Models\EveEntity;
use NicolasKion\Esi\DTO\UniverseId;
use NicolasKion\Esi\DTO\UniverseIds;
use NicolasKion\Esi\Esi;

final class EveEntityResolver
{
    private const int MAX_NAMES_PER_REQUEST = 500;

    public function __construct(private readonly Esi $esi) {}

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
            $result = $this->esi->getIds($chunk);

            if ($result->failed()) {
                continue;
            }

            /** @var UniverseIds $data */
            $data = $result->data;

            $resolvedCharacters = $this->toMap($data->characters);
            $resolvedTypes = $this->toMap($data->inventory_types);

            $rows = [];

            foreach (array_intersect($characterNames, $chunk) as $name) {
                // Drone and structure names occupy the pilot slot in the log.
                // If ESI knows the name as an item type, that wins: a stray
                // player who happens to share a drone's name must not get
                // their portrait rendered on drone rows.
                $isItemType = isset($resolvedTypes[$name]);

                $rows[$name.'|character'] = [
                    'kind' => EveEntityKind::Character->value,
                    'name' => $name,
                    'eve_id' => $isItemType ? null : ($resolvedCharacters[$name] ?? null),
                ];

                if ($isItemType) {
                    $rows[$name.'|type'] = [
                        'kind' => EveEntityKind::InventoryType->value,
                        'name' => $name,
                        'eve_id' => $resolvedTypes[$name],
                    ];
                }
            }

            foreach (array_intersect($typeNames, $chunk) as $name) {
                $rows[$name.'|type'] = [
                    'kind' => EveEntityKind::InventoryType->value,
                    'name' => $name,
                    'eve_id' => $resolvedTypes[$name] ?? null,
                ];
            }

            EveEntity::query()->upsert(array_values($rows), ['kind', 'name'], ['eve_id']);

            $knownCharacters = [...$knownCharacters, ...array_intersect_key($resolvedCharacters, array_flip($characterNames))];
            $knownTypes = [...$knownTypes, ...$resolvedTypes];
        }

        return [
            'characters' => $this->buildMap($characterNames, array_diff_key($knownCharacters, $knownTypes)),
            'types' => $this->buildMap($typeNames, $knownTypes),
        ];
    }

    /**
     * @param  UniverseId[]  $ids
     * @return array<string, int>
     */
    private function toMap(array $ids): array
    {
        $map = [];

        foreach ($ids as $id) {
            $map[$id->name] = $id->id;
        }

        return $map;
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
