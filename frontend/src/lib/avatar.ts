// The pilot avatar's portrait or, for NPCs and unresolved names, its
// initials on a hue picked by a hash of the name
// (resources/js/components/ui/PilotAvatar.vue).

const HUES = [
  'bg-zinc-700 text-zinc-200',
  'bg-stone-700 text-stone-200',
  'bg-zinc-600 text-zinc-100',
  'bg-neutral-700 text-neutral-200',
  'bg-stone-600 text-stone-100',
];

export function portraitUrl(characterId: number | null): string | null {
  return characterId
    ? `https://images.evetech.net/characters/${characterId}/portrait?size=64`
    : null;
}

export function shipIconUrl(typeId: number | null): string | null {
  return typeId ? `https://images.evetech.net/types/${typeId}/icon?size=32` : null;
}

export function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]!.toUpperCase())
    .join('');
}

export function hueClass(name: string): string {
  let hash = 0;

  for (const char of name) {
    hash = (hash * 31 + char.charCodeAt(0)) | 0;
  }

  return HUES[Math.abs(hash) % HUES.length];
}
