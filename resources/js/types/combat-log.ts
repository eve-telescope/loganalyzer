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
    /** ISO datetime: YYYY-MM-DDTHH:MM:SS — primary identifier, also used in URL state. */
    datetime: string;
    /** HH:MM:SS — display label for the chart x-axis. */
    label: string;
    dpsDealt: number;
    dpsReceived: number;
    logiReceived: number;
    logiDealt: number;
    neutIn: number;
    neutOut: number;
}

export interface DateTimeRange {
    start: string;
    end: string;
}

export type SeriesKey =
    | 'dpsDealt'
    | 'dpsReceived'
    | 'logiReceived'
    | 'logiDealt'
    | 'neutIn'
    | 'neutOut';

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
    type: 'damage' | 'logistics' | 'neutralization';
}

export interface CombatAnalysis {
    listener: string;
    sessionStarted: string;
    events: CombatEventData[];
}
