<?php

// What PHP and Laravel return for the string functions the port
// depends on, over the inputs in tests/fixtures/php-functions/inputs.json
// (base64). The Rust ports are tested against the answers
// (tests/integration/php_functions.rs).
//
//   run.sh php php-functions.php < inputs.json > answers.json

declare(strict_types=1);

require '/app/vendor/autoload.php';

use Illuminate\Support\Str;
use Symfony\Component\HttpFoundation\HeaderUtils;
use Symfony\Component\HttpFoundation\File\UploadedFile;

$inputs = array_map(
    fn (string $encoded): string => base64_decode($encoded, true),
    json_decode(stream_get_contents(STDIN), true, flags: JSON_THROW_ON_ERROR),
);

$encode = fn (?string $value): ?string => $value === null ? null : base64_encode($value);

$answers = [];

foreach ($inputs as $input) {
    $valid = mb_check_encoding($input, 'UTF-8');
    $disposition = null;

    if ($valid && $input !== '') {
        try {
            $fallback = str_replace('%', '', Str::ascii($input));
            $disposition = HeaderUtils::makeDisposition('attachment', $input, $fallback);
        } catch (Throwable $error) {
            $disposition = 'error: '.$error->getMessage();
        }
    }

    $uploaded = $valid ? new UploadedFile(__FILE__, $input, null, UPLOAD_ERR_OK, true) : null;

    $answers[] = [
        'input' => base64_encode($input),
        'strip_tags' => $encode(strip_tags($input)),
        'mb_trim' => $valid ? $encode(mb_trim($input)) : null,
        'trim' => $encode(trim($input)),
        'str_trim' => $valid ? $encode(Str::trim($input)) : null,
        'int' => preg_match('/^\d+$/', $input) ? (int) $input : null,
        'client_name' => $uploaded ? $encode($uploaded->getClientOriginalName()) : null,
        'client_extension' => $uploaded ? $encode(strtolower($uploaded->getClientOriginalExtension())) : null,
        'disposition' => $disposition === null ? null : $encode($disposition),
    ];
}

echo json_encode($answers, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR), "\n";
