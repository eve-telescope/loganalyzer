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

/**
 * Human relative time ("3 weeks ago") for an EVE combat log timestamp.
 * EVE gamelog times are UTC.
 */
export function relativeTimeFromEve(timestamp: string): string | null {
    const match = timestamp.match(
        /^(\d{4})\.(\d{2})\.(\d{2}) (\d{2}):(\d{2}):(\d{2})$/,
    );

    if (!match) {
        return null;
    }

    const [, y, mo, d, h, mi, se] = match;
    const then = Date.UTC(+y, +mo - 1, +d, +h, +mi, +se);
    const seconds = Math.round((then - Date.now()) / 1000);

    const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
        ['year', 31536000],
        ['month', 2592000],
        ['week', 604800],
        ['day', 86400],
        ['hour', 3600],
        ['minute', 60],
    ];

    const formatter = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

    for (const [unit, size] of UNITS) {
        if (Math.abs(seconds) >= size) {
            return formatter.format(Math.trunc(seconds / size), unit);
        }
    }

    return 'just now';
}
