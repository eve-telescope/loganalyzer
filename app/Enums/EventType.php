<?php

declare(strict_types=1);

namespace App\Enums;

enum EventType: string
{
    case Damage = 'damage';
    case Logistics = 'logistics';
}
