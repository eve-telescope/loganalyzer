<?php

declare(strict_types=1);

use App\Http\Controllers\CombatLogController;
use Illuminate\Support\Facades\Route;

Route::get('/', [CombatLogController::class, 'create'])->name('home');
Route::post('/analyze', [CombatLogController::class, 'store'])->name('combat-log.analyze');
Route::post('/logs/{combatLog}/regenerate', [CombatLogController::class, 'regenerate'])->name('combat-log.regenerate');
Route::get('/logs/{combatLog}', [CombatLogController::class, 'show'])->name('combat-log.show');
Route::get('/logs/{combatLog}/download', [CombatLogController::class, 'download'])->name('combat-log.download');
