<?php

declare(strict_types=1);

use App\Models\CombatLog;
use App\Models\EveEntity;
use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\Http;
use Illuminate\Support\Facades\Storage;

beforeEach(function () {
    Http::preventStrayRequests();
});

function fakeEsiIds(): void
{
    Http::fake([
        'esi.evetech.net/*' => Http::response([
            'characters' => [
                ['id' => 93000001, 'name' => 'Aria Vex'],
                ['id' => 93000002, 'name' => 'Korr Malan'],
            ],
            'inventory_types' => [
                ['id' => 24698, 'name' => 'Drake'],
                ['id' => 24702, 'name' => 'Hurricane'],
            ],
        ]),
    ]);
}

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
    fakeEsiIds();

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
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
    fakeEsiIds();

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    $this->get("/logs/{$combatLog->uuid}?from=2026-02-12T10:20:00&to=2026-02-12T10:25:00&hide=logiDealt,neutOut&pilot=Aria%20Vex")
        ->assertStatus(200)
        ->assertInertia(fn ($page) => $page
            ->component('Analysis')
            ->where('filters.from', '2026-02-12T10:20:00')
            ->where('filters.to', '2026-02-12T10:25:00')
            ->where('filters.hide', ['logiDealt', 'neutOut'])
            ->where('filters.pilot', 'Aria Vex')
            ->etc()
        );
});

test('the show page returns empty filters when no query params are present', function () {
    fakeEsiIds();

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    $this->get("/logs/{$combatLog->uuid}")
        ->assertStatus(200)
        ->assertInertia(fn ($page) => $page
            ->where('filters.from', null)
            ->where('filters.to', null)
            ->where('filters.pilot', null)
            ->where('filters.hide', [])
            ->etc()
        );
});

test('the show page renders with analysis data', function () {
    fakeEsiIds();

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
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
    fakeEsiIds();

    $maxKb = (int) config('loganalyzer.upload.max_size_mb') * 1024;
    $file = UploadedFile::fake()->create('huge.txt', $maxKb + 1);

    $this->post('/analyze', ['log_file' => $file])
        ->assertSessionHasErrors('log_file');
});

test('combat events are stored as individual rows', function () {
    fakeEsiIds();

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();
    $eventCount = $combatLog->events()->count();

    expect($eventCount)->toBeGreaterThan(0);
});

test('the log is shareable via uuid url', function () {
    fakeEsiIds();

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    $this->get("/logs/{$combatLog->uuid}")
        ->assertStatus(200);
});

test('uploading resolves pilot and ship names to EVE ids via ESI', function () {
    fakeEsiIds();

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    Http::assertSent(fn ($request) => str_contains($request->url(), '/universe/ids/'));

    expect(EveEntity::query()->where('name', 'Aria Vex')->value('eve_id'))->toBe(93000001)
        ->and(EveEntity::query()->where('name', 'Drake')->value('eve_id'))->toBe(24698);
});

test('the show page passes resolved EVE ids as props', function () {
    fakeEsiIds();

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    $this->get("/logs/{$combatLog->uuid}")
        ->assertStatus(200)
        ->assertInertia(fn ($page) => $page
            ->where('pilotIds.Aria Vex', 93000001)
            ->where('shipTypeIds.Drake', 24698)
            ->etc()
        );
});

test('names already resolved are not requested from ESI again', function () {
    fakeEsiIds();

    EveEntity::factory()->create(['name' => 'Aria Vex', 'eve_id' => 93000001]);

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    Http::assertSent(function ($request) {
        return str_contains($request->url(), '/universe/ids/')
            && ! in_array('Aria Vex', $request->data(), true);
    });
});

test('uploading still succeeds when ESI is unavailable', function () {
    config()->set('esi.retry_policy', ['tries' => 1, 'delay' => 0]);

    Http::fake(['esi.evetech.net/*' => Http::response(null, 500)]);

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
    );

    $response = $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    expect($combatLog)->not->toBeNull();
    $response->assertRedirect("/logs/{$combatLog->uuid}");

    expect(EveEntity::query()->count())->toBe(0);
});

test('names matching an item type are never treated as characters', function () {
    // A drone in the pilot slot whose name also belongs to a real player.
    Http::fake([
        'esi.evetech.net/*' => Http::response([
            'characters' => [
                ['id' => 95194769, 'name' => 'Hammerhead II'],
                ['id' => 93000001, 'name' => 'Aria Vex'],
            ],
            'inventory_types' => [
                ['id' => 2185, 'name' => 'Hammerhead II'],
            ],
        ]),
    ]);

    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Nicolas Kion
      Session Started: 2026.02.12 09:41:44
    ------------------------------------------------------------
    [ 2026.02.12 09:42:00 ] (combat) <color=0xff00ffff><b>120</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Hammerhead II</b><font size=10><color=0x77ffffff> - Light Neutron Blaster II - Hits
    [ 2026.02.12 09:42:05 ] (combat) <color=0xff00ffff><b>340</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Aria Vex[RAZR](Drake)</b><font size=10><color=0x77ffffff> - Heavy Missile - Hits
    LOG;

    $file = UploadedFile::fake()->createWithContent('combat.txt', $log);

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    expect(EveEntity::query()->where('kind', 'character')->where('name', 'Hammerhead II')->value('eve_id'))->toBeNull()
        ->and(EveEntity::query()->where('kind', 'inventory_type')->where('name', 'Hammerhead II')->value('eve_id'))->toBe(2185);

    $this->get("/logs/{$combatLog->uuid}")
        ->assertStatus(200)
        ->assertInertia(fn ($page) => $page
            ->missing('pilotIds.Hammerhead II')
            ->where('shipTypeIds.Hammerhead II', 2185)
            ->where('pilotIds.Aria Vex', 93000001)
            ->etc()
        );
});

test('uploading stores the compressed raw log and it can be downloaded', function () {
    fakeEsiIds();
    Storage::fake();

    $contents = file_get_contents(base_path('tests/Fixtures/testlog.txt'));
    $file = UploadedFile::fake()->createWithContent('my-fight.txt', $contents);

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();

    Storage::assertExists("combat-logs/{$combatLog->uuid}.txt.gz");
    expect(gzdecode(Storage::get("combat-logs/{$combatLog->uuid}.txt.gz")))->toBe($contents);

    $this->get("/logs/{$combatLog->uuid}")
        ->assertInertia(fn ($page) => $page->where('rawLogAvailable', true)->etc());

    $response = $this->get("/logs/{$combatLog->uuid}/download");
    $response->assertStatus(200);
    $response->assertDownload('my-fight.txt');
    expect($response->streamedContent())->toBe($contents);
});

test('the download route returns 404 when no raw log is stored', function () {
    fakeEsiIds();
    Storage::fake();

    $file = UploadedFile::fake()->createWithContent(
        'combat.txt',
        file_get_contents(base_path('tests/Fixtures/testlog.txt')),
    );

    $this->post('/analyze', ['log_file' => $file]);

    $combatLog = CombatLog::first();
    Storage::delete("combat-logs/{$combatLog->uuid}.txt.gz");

    $this->get("/logs/{$combatLog->uuid}")
        ->assertInertia(fn ($page) => $page->where('rawLogAvailable', false)->etc());

    $this->get("/logs/{$combatLog->uuid}/download")->assertStatus(404);
});
