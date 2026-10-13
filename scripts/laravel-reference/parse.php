<?php

// The PHP parser's answer for log files, the expected outputs of the
// Rust parser (tests/integration/parser.rs). Runs in the reference
// checkout (run.sh php), warnings turned into exceptions as Laravel's
// error handler does:
//
//   run.sh php parse.php < inputs.json > outputs.json
//
// Input: a JSON object {name: base64 of the file's bytes}. Output: {name:
// {listener, sessionStarted, events}} or {name: {error}} when the parser
// threw (which answered the upload with a 500).

declare(strict_types=1);

require '/app/vendor/autoload.php';

set_error_handler(function (int $severity, string $message, string $file, int $line): bool {
    throw new ErrorException($message, 0, $severity, $file, $line);
});

$parser = new App\Services\CombatLogParser;
$inputs = json_decode(stream_get_contents(STDIN), true, flags: JSON_THROW_ON_ERROR);
$outputs = [];

foreach ($inputs as $name => $encoded) {
    try {
        $outputs[$name] = $parser->parse(base64_decode($encoded, true));
    } catch (Throwable $error) {
        $outputs[$name] = ['error' => get_class($error).': '.$error->getMessage()];
    }
}

echo json_encode($outputs, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR), "\n";
