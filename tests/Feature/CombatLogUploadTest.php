<?php

declare(strict_types=1);

use App\Models\CombatLog;
use Illuminate\Http\UploadedFile;

test('the upload page renders with the configured max upload size', function () {
    config()->set('loganalyzer.upload.max_size_mb', 7);

    $this->get('/')
        ->assertStatus(200)
        ->assertInertia(fn ($page) => $page
            ->component('Upload')
            ->where('maxUploadSizeMb', 7)
        );
});

test('a combat log can be uploaded and redirects to show page', function () {
    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(storage_path('app/private/testlog.txt')),
    );

    $response = $this->post('/analyze', [
        'log_file' => $file,
    ]);

    $combatLog = CombatLog::first();

    expect($combatLog)->not->toBeNull();
    expect($combatLog->listener)->toBe('Nicolas Kion');
    expect($combatLog->events)->not->toBeEmpty();

    $response->assertRedirect("/logs/{$combatLog->uuid}");
});

test('the show page passes URL filters as a prop', function () {
    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(storage_path('app/private/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    $this->get("/logs/{$combatLog->uuid}?from=2026-02-12T10:20:00&to=2026-02-12T10:25:00&hide=logiDealt,neutOut")
        ->assertStatus(200)
        ->assertInertia(fn ($page) => $page
            ->component('Analysis')
            ->where('filters.from', '2026-02-12T10:20:00')
            ->where('filters.to', '2026-02-12T10:25:00')
            ->where('filters.hide', ['logiDealt', 'neutOut'])
            ->etc()
        );
});

test('the show page returns empty filters when no query params are present', function () {
    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(storage_path('app/private/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    $this->get("/logs/{$combatLog->uuid}")
        ->assertStatus(200)
        ->assertInertia(fn ($page) => $page
            ->where('filters.from', null)
            ->where('filters.to', null)
            ->where('filters.hide', [])
            ->etc()
        );
});

test('the show page renders with analysis data', function () {
    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(storage_path('app/private/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    $this->get("/logs/{$combatLog->uuid}")
        ->assertStatus(200)
        ->assertInertia(fn ($page) => $page
            ->component('Analysis')
            ->has('analysis.listener')
            ->has('analysis.events')
            ->has('uuid')
        );
});

test('uploading without a file returns validation error', function () {
    $this->post('/analyze', [])
        ->assertSessionHasErrors('log_file');
});

test('uploading an oversized file returns validation error', function () {
    $maxKb = (int) config('loganalyzer.upload.max_size_mb') * 1024;
    $file = UploadedFile::fake()->create('huge.txt', $maxKb + 1);

    $this->post('/analyze', ['log_file' => $file])
        ->assertSessionHasErrors('log_file');
});

test('combat events are stored as individual rows', function () {
    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(storage_path('app/private/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();
    $eventCount = $combatLog->events()->count();

    expect($eventCount)->toBeGreaterThan(0);
});

test('the log is shareable via uuid url', function () {
    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(storage_path('app/private/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    $this->get("/logs/{$combatLog->uuid}")
        ->assertStatus(200);
});
