<?php

return [
    'enabled' => env('ZVAULT_ENABLED', true),
    'token' => env('ZVAULT_TOKEN', ''),
    'org_id' => env('ZVAULT_ORG_ID', ''),
    'project_id' => env('ZVAULT_PROJECT_ID', ''),
    'env' => env('ZVAULT_ENV', 'production'),
    'base_url' => env('ZVAULT_URL', 'https://api.zvault.cloud'),
    'inject_env' => env('ZVAULT_INJECT_ENV', false),
];
