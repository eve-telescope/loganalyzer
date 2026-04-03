export interface CombatSummary {
    totalDamageDealt: number;
    totalDamageReceived: number;
    combatDurationSeconds: number;
    dpsDealt: number;
    dpsReceived: number;
    totalOutgoingHits: number;
    totalOutgoingMisses: number;
    totalIncomingHits: number;
    totalIncomingMisses: number;
    totalLogiReceived: number;
    totalLogiDealt: number;
}

export interface DpsDataPoint {
    timestamp: string;
    dpsDealt: number;
    dpsReceived: number;
    logiReceived: number;
    logiDealt: number;
}

export interface TargetDamage {
    damage: number;
    hits: number;
    misses: number;
    ship: string | null;
}

export interface WeaponDamage {
    damage: number;
    hits: number;
}

export interface CombatEventData {
    timestamp: string;
    damage: number;
    direction: 'outgoing' | 'incoming';
    playerName: string;
    corporation: string | null;
    shipName: string | null;
    weapon: string;
    quality: string;
    type: 'damage' | 'logistics';
}

export interface CombatAnalysis {
    listener: string;
    sessionStarted: string;
    events: CombatEventData[];
}

export interface TimeRange {
    startIndex: number;
    endIndex: number;
}
