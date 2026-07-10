<?php

declare(strict_types=1);

use App\Enums\EventDirection;
use App\Enums\EventType;
use App\Services\CombatLogParser;

beforeEach(function () {
    $this->parser = new CombatLogParser;
    $this->logContents = file_get_contents(base_path('tests/Fixtures/testlog.txt'));
});

test('it extracts the listener name from the header', function () {
    $result = $this->parser->parse($this->logContents);

    expect($result['listener'])->toBe('Nicolas Kion');
});

test('it extracts the session start time', function () {
    $result = $this->parser->parse($this->logContents);

    expect($result['sessionStarted'])->toBe('2026.02.12 09:41:44');
});

test('it parses combat events', function () {
    $result = $this->parser->parse($this->logContents);

    expect($result['events'])->toBeArray()->not->toBeEmpty();
});

test('it parses outgoing damage hits correctly', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:05 ] (combat) <color=0xff00ffff><b>500</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Enemy Pilot[CORP](Drake)</b><font size=10><color=0x77ffffff> - Heavy Missile - Hits
    LOG;

    $result = $this->parser->parse($log);

    expect($result['events'])->toHaveCount(1);

    $event = $result['events'][0];
    expect($event->damage)->toBe(500);
    expect($event->direction)->toBe(EventDirection::Outgoing);
    expect($event->playerName)->toBe('Enemy Pilot');
    expect($event->shipName)->toBe('Drake');
    expect($event->weapon)->toBe('Heavy Missile');
    expect($event->type)->toBe(EventType::Damage);
});

test('it parses incoming miss lines', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:05 ] (combat) Hammerhead II belonging to Enemy Pilot misses you completely - Hammerhead II
    LOG;

    $result = $this->parser->parse($log);

    expect($result['events'])->toHaveCount(1);

    $event = $result['events'][0];
    expect($event->damage)->toBe(0);
    expect($event->direction)->toBe(EventDirection::Incoming);
    expect($event->quality)->toBe('Misses');
});

test('it parses outgoing miss lines', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:05 ] (combat) Your Heavy Missile misses Enemy Pilot completely - Heavy Missile
    LOG;

    $result = $this->parser->parse($log);

    $event = $result['events'][0];
    expect($event->direction)->toBe(EventDirection::Outgoing);
    expect($event->quality)->toBe('Misses');
});

test('it handles empty combat logs gracefully', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:05 ] (notify) Ship stopping
    LOG;

    $result = $this->parser->parse($log);

    expect($result['events'])->toBeEmpty();
});

test('it parses logistics received events', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:05 ] (combat) <color=0xffccff66><b>575</b><color=0x77ffffff><font size=10> remote shield boosted by </font><b><color=0xffffffff><font size=10><color=0xFFFF7040><b>Basilisk</b></color></font> <font size=10>Logi Pilot</font></b><color=0x77ffffff><font size=10> - Large Remote Shield Booster</font>
    LOG;

    $result = $this->parser->parse($log);

    expect($result['events'])->toHaveCount(1);

    $event = $result['events'][0];
    expect($event->damage)->toBe(575);
    expect($event->direction)->toBe(EventDirection::Incoming);
    expect($event->type)->toBe(EventType::Logistics);
    expect($event->playerName)->toBe('Logi Pilot');
    expect($event->shipName)->toBe('Basilisk');
    expect($event->weapon)->toBe('Large Remote Shield Booster');
});

test('it parses logi events from the test log file', function () {
    $result = $this->parser->parse($this->logContents);

    $logiEvents = array_filter($result['events'], fn ($e) => $e->type === EventType::Logistics);
    expect(count($logiEvents))->toBeGreaterThan(0);
});

test('it parses incoming energy neutralization events', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:05 ] (combat) <color=0xff7fffff><b>522 GJ</b><color=0x77ffffff><font size=10> energy neutralized </font><b><color=0xffffffff><fontsize=12><color=0xFF2ecc71><b>Sabre</b></color></fontsize></b><color=0x77ffffff><font size=10> - True Sansha Heavy Energy Neutralizer</font>
    LOG;

    $result = $this->parser->parse($log);

    expect($result['events'])->toHaveCount(1);

    $event = $result['events'][0];
    expect($event->damage)->toBe(522);
    expect($event->direction)->toBe(EventDirection::Incoming);
    expect($event->type)->toBe(EventType::Neutralization);
    expect($event->shipName)->toBe('Sabre');
    expect($event->weapon)->toBe('True Sansha Heavy Energy Neutralizer');
    expect($event->quality)->toBe('Neutralized');
});

test('it parses incoming energy nosferatu events with negative GJ', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:05 ] (combat) <color=0xffe57f7f><b>-2 GJ</b><color=0x77ffffff><font size=10> energy drained to </font><b><color=0xffffffff><fontsize=12><color=0xFF2ecc71><b>Stiletto</b></color></fontsize></b><color=0x77ffffff><font size=10> - Small Ghoul Compact Energy Nosferatu</font>
    LOG;

    $result = $this->parser->parse($log);

    expect($result['events'])->toHaveCount(1);

    $event = $result['events'][0];
    expect($event->damage)->toBe(2);
    expect($event->direction)->toBe(EventDirection::Incoming);
    expect($event->type)->toBe(EventType::Neutralization);
    expect($event->shipName)->toBe('Stiletto');
    expect($event->weapon)->toBe('Small Ghoul Compact Energy Nosferatu');
    expect($event->quality)->toBe('Drained');
});

test('it parses neutralization events from the testlog2 fixture', function () {
    $contents = file_get_contents(base_path('tests/Fixtures/testlog2.txt'));
    $result = $this->parser->parse($contents);

    $neutEvents = array_filter(
        $result['events'],
        fn ($e) => $e->type === EventType::Neutralization,
    );

    expect(count($neutEvents))->toBeGreaterThan(800);
});

test('events use enums for direction and type', function () {
    $result = $this->parser->parse($this->logContents);

    foreach ($result['events'] as $event) {
        expect($event->direction)->toBeInstanceOf(EventDirection::class);
        expect($event->type)->toBeInstanceOf(EventType::class);
    }
});

test('it attributes outgoing drone damage to the sole owner', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:01 ] (combat) <color=0xffcc0000><b>88</b> <color=0x77ffffff><font size=10>from</font> <b><color=0xffffffff>Tom DaNanRen[.BOP](Cenotaph)</b><font size=10><color=0x77ffffff> - Hammerhead II - Penetrates
    [ 2026.01.01 12:00:05 ] (combat) <color=0xff00ffff><b>270</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Hammerhead II[.BOP](Hammerhead II)</b><font size=10><color=0x77ffffff> - Heavy Missile - Hits
    LOG;

    $result = $this->parser->parse($log);

    expect($result['events'])->toHaveCount(2);

    $droneHit = $result['events'][1];
    expect($droneHit->playerName)->toBe('Tom DaNanRen')
        ->and($droneHit->shipName)->toBe('Hammerhead II')
        ->and($droneHit->direction)->toBe(EventDirection::Outgoing);
});

test('it leaves drone damage unattributed when several pilots own that drone type', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:01 ] (combat) <color=0xffcc0000><b>88</b> <color=0x77ffffff><font size=10>from</font> <b><color=0xffffffff>Pilot One[.BOP](Cenotaph)</b><font size=10><color=0x77ffffff> - Warrior II - Penetrates
    [ 2026.01.01 12:00:02 ] (combat) <color=0xffcc0000><b>92</b> <color=0x77ffffff><font size=10>from</font> <b><color=0xffffffff>Pilot Two[.BOP](Rupture)</b><font size=10><color=0x77ffffff> - Warrior II - Hits
    [ 2026.01.01 12:00:05 ] (combat) <color=0xff00ffff><b>120</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Warrior II[.BOP](Warrior II)</b><font size=10><color=0x77ffffff> - Heavy Missile - Hits
    LOG;

    $result = $this->parser->parse($log);

    $droneHit = $result['events'][2];
    expect($droneHit->playerName)->toBe('Warrior II');
});

test('it strips markup from logistics dealt weapons', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:05 ] (combat) <color=0xffccff66><b>410</b><color=0x77ffffff><font size=10> remote shield boosted to </font><b><color=0xffffffff>Fleet Mate</b><color=0x77ffffff><font size=10> - Medium Remote Shield Booster</font>
    LOG;

    $result = $this->parser->parse($log);

    expect($result['events'])->toHaveCount(1);

    $event = $result['events'][0];
    expect($event->weapon)->toBe('Medium Remote Shield Booster')
        ->and($event->playerName)->toBe('Fleet Mate')
        ->and($event->type)->toBe(EventType::Logistics);
});

test('no parsed field ever contains markup', function () {
    $contents = file_get_contents(base_path('tests/Fixtures/testlog.txt'));
    $result = $this->parser->parse($contents);

    foreach ($result['events'] as $event) {
        expect($event->playerName)->not->toContain('<')
            ->and($event->weapon)->not->toContain('<')
            ->and($event->shipName ?? '')->not->toContain('<');
    }
});

test('it parses logistics received with a plain pilot target (issue #1)', function () {
    $log = <<<'LOG'
    ------------------------------------------------------------
      Gamelog
      Listener: Test Pilot
      Session Started: 2026.01.01 12:00:00
    ------------------------------------------------------------
    [ 2026.01.01 12:00:05 ] (combat) <color=0xffccff66><b>512</b><color=0x77ffffff><font size=10> remote shield boosted by </font><b><color=0xffffffff>Logi Pilot[CORP](Basilisk)</b><color=0x77ffffff><font size=10> - Large Remote Shield Booster</font>
    LOG;

    $result = $this->parser->parse($log);

    expect($result['events'])->toHaveCount(1);

    $event = $result['events'][0];
    expect($event->type)->toBe(EventType::Logistics)
        ->and($event->direction)->toBe(EventDirection::Incoming)
        ->and($event->damage)->toBe(512)
        ->and($event->playerName)->toBe('Logi Pilot')
        ->and($event->corporation)->toBe('CORP')
        ->and($event->shipName)->toBe('Basilisk')
        ->and($event->weapon)->toBe('Large Remote Shield Booster');
});
