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
        Schema::create('combat_logs', function (Blueprint $table): void {
            $table->id();
            $table->uuid('uuid')->unique();
            $table->string('listener');
            $table->string('session_started');
            $table->string('original_filename')->nullable();
            $table->timestamps();
        });

        Schema::create('combat_events', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('combat_log_id')->constrained()->cascadeOnDelete();
            $table->string('timestamp');
            $table->unsignedInteger('damage');
            $table->string('direction');
            $table->string('player_name');
            $table->string('corporation')->nullable();
            $table->string('ship_name')->nullable();
            $table->string('weapon');
            $table->string('quality');
            $table->string('type')->default('damage');
            $table->index('combat_log_id');
        });
    }

    /**
     * Reverse the migrations.
     */
    public function down(): void
    {
        Schema::dropIfExists('combat_events');
        Schema::dropIfExists('combat_logs');
    }
};
