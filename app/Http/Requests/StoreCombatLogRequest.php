<?php

declare(strict_types=1);

namespace App\Http\Requests;

use Illuminate\Contracts\Validation\ValidationRule;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rules\File;

final class StoreCombatLogRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /**
     * @return array<string, ValidationRule|array<mixed>|string>
     */
    public function rules(): array
    {
        $maxSizeMb = (int) config('loganalyzer.upload.max_size_mb');

        return [
            'log_file' => ['required', File::default()->extensions(['txt', 'log'])->max($maxSizeMb * 1024)],
        ];
    }
}
