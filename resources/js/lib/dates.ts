// Swedish locale produces ISO-8601 style "YYYY-MM-DD HH:MM:SS" / "HH:MM:SS"
// in local time, sidestepping manual zero-padding.
const ISO_LOCALE = 'sv-SE';

export function formatTime(ms: number): string {
    return new Date(ms).toLocaleTimeString(ISO_LOCALE);
}

export function formatDateTime(ms: number): string {
    return new Date(ms).toLocaleString(ISO_LOCALE).replace(' ', 'T');
}

/**
 * Convert an EVE combat log timestamp ("YYYY.MM.DD HH:MM:SS") to ISO 8601
 * datetime ("YYYY-MM-DDTHH:MM:SS") for lexicographic comparison.
 */
export function eveTimestampToIso(timestamp: string): string {
    return timestamp.replace(/\./g, '-').replace(' ', 'T');
}
