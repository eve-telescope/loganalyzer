<?php

declare(strict_types=1);

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    /**
     * Run the migrations.
     */
    public function up(): void
    {
        Schema::create('eve_entities', function (Blueprint $table) {
            $table->id();
            $table->string('kind');
            $table->string('name');
            $table->unsignedBigInteger('eve_id')->nullable();
            $table->timestamps();

            $table->unique(['kind', 'name']);
        });
    }

    /**
     * Reverse the migrations.
     */
    public function down(): void
    {
        Schema::dropIfExists('eve_entities');
    }
};
