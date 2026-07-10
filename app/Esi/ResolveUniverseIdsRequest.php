<?php

declare(strict_types=1);

namespace App\Esi;

use Illuminate\Http\Client\Response;
use NicolasKion\Esi\Enums\RequestMethod;
use NicolasKion\Esi\Interfaces\WithBody;
use NicolasKion\Esi\Request;

/**
 * Bulk-resolves names to EVE IDs via POST /universe/ids/.
 *
 * The response groups matches by category (characters, inventory_types,
 * corporations, ...); names without a match are simply absent.
 */
final class ResolveUniverseIdsRequest extends Request implements WithBody
{
    /**
     * @param  list<string>  $names  Unique names, at most 500 per request.
     */
    public function __construct(public array $names) {}

    public function resolveEndpoint(): string
    {
        return '/universe/ids/';
    }

    public function getMethod(): RequestMethod
    {
        return RequestMethod::POST;
    }

    public function getBody(): array
    {
        return $this->names;
    }

    /**
     * @return array<string, list<array{id: int, name: string}>>
     */
    public function createDto(Response $response, mixed $data): array
    {
        return is_array($data) ? $data : [];
    }
}
