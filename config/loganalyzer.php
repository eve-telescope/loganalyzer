<?php

declare(strict_types=1);

return [

    /*
    |--------------------------------------------------------------------------
    | Combat Log Upload
    |--------------------------------------------------------------------------
    |
    | Maximum allowed combat log upload size in megabytes. Enforced both
    | server-side (StoreCombatLogRequest) and client-side (FileDropZone)
    | so that oversized files are rejected before the request is sent.
    |
    | Make sure your web server (e.g. NGINX `client_max_body_size`) and PHP
    | (`post_max_size`, `upload_max_filesize`) are set at least this high.
    |
    */

    'upload' => [
        'max_size_mb' => (int) env('LOGANALYZER_UPLOAD_MAX_SIZE_MB', 25),
    ],

];
