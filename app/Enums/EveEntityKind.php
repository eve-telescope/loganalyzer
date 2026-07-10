<?php

declare(strict_types=1);

namespace App\Enums;

enum EveEntityKind: string
{
    case Character = 'character';
    case InventoryType = 'inventory_type';
}
