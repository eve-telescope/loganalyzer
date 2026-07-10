<?php

declare(strict_types=1);

namespace App\Models;

use App\Enums\EveEntityKind;
use Database\Factories\EveEntityFactory;
use Illuminate\Database\Eloquent\Attributes\Fillable;
use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;

#[Fillable(['kind', 'name', 'eve_id'])]
final class EveEntity extends Model
{
    /** @use HasFactory<EveEntityFactory> */
    use HasFactory;

    protected function casts(): array
    {
        return [
            'kind' => EveEntityKind::class,
        ];
    }
}
