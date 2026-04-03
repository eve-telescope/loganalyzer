<?php

declare(strict_types=1);

use App\Http\Controllers\CombatLogController;
use Illuminate\Support\Facades\Route;

Route::get('/', [CombatLogController::class, 'create'])->name('home');
Route::post('/analyze', [CombatLogController::class, 'store'])->name('combat-log.analyze');
Route::get('/logs/{combatLog}', [CombatLogController::class, 'show'])->name('combat-log.show');
