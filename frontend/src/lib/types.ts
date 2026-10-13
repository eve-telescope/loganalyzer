// The props the api serves for the two pages (`/api/pages/*`).

export interface DpsDataPoint {
  /** Bucket start as ISO 8601 local datetime ("2026-02-12T09:41:44"). */
  datetime: string;
  /** Display label for the chart x-axis. */
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

export interface Filters {
  from: string | null;
  to: string | null;
  pilot: string | null;
  hide: string[];
}

/** The props every page gets. */
export interface SharedProps {
  name: string;
  auth: { user: null };
}

export interface UploadProps extends SharedProps {
  maxUploadSizeMb: number;
}

export interface AnalysisProps extends SharedProps {
  analysis: CombatAnalysis;
  uuid: string;
  rawLogAvailable: boolean;
  pilotIds: Record<string, number>;
  shipTypeIds: Record<string, number>;
  filters: Filters;
}
