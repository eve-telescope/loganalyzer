<?php

// The package's config/esi.php (nicolaskion/eve v0.6) with the base URL
// from the environment, so the reference resolves names against
// mock_esi.py. Everything else is the package's.
return [
    'user_agent' => env('APP_NAME'),
    'client_id' => env('EVE_CLIENT_ID'),
    'client_secret' => env('EVE_CLIENT_SECRET'),
    'retry_policy' => [
        'tries' => 5,
        'delay' => 5000,
    ],
    'base_url' => env('ESI_BASE_URL', 'https://esi.evetech.net'),
    'compatibility_date' => '2026-05-19',
];
