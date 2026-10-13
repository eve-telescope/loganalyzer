import { describe, expect, it } from 'vitest';

import { analysisHead, uploadHead } from './seo.ts';

const url = new URL('https://loganalyzer.eve-telescope.com/logs/abc?pilot=Aria%20Vex');

function tags(head: ReturnType<typeof uploadHead>): Record<string, string> {
  return Object.fromEntries(head.meta.map((tag) => [tag.key, tag.content]));
}

describe('the page heads', () => {
  it('names the upload page', () => {
    const head = uploadHead('EVE Log Analyzer', new URL('https://loganalyzer.eve-telescope.com/'));
    expect(head.title).toBe('Combat Log Analyzer - EVE Log Analyzer');
    expect(tags(head)).toEqual({
      'theme-color': '#09090b',
      'og:url': 'https://loganalyzer.eve-telescope.com',
      'og:image': 'https://loganalyzer.eve-telescope.com/og-image.png',
      'og:image:width': '1200',
      'og:image:height': '630',
      'og:image:alt': 'Combat Log Analyzer — EVE Online after-action reports',
      'twitter:image': 'https://loganalyzer.eve-telescope.com/og-image.png',
      description:
        'Upload and analyze EVE Online combat logs. DPS graphs, damage breakdowns, logistics, and detailed pilot statistics.',
      'og:type': 'website',
      'og:site_name': 'Combat Log Analyzer',
      'og:title': 'Combat Log Analyzer',
      'og:description':
        'Upload and analyze EVE Online combat logs. DPS graphs, damage breakdowns, logistics, and detailed pilot statistics.',
      'twitter:card': 'summary_large_image',
      'twitter:title': 'Combat Log Analyzer',
      'twitter:description':
        'Upload and analyze EVE Online combat logs. DPS graphs, damage breakdowns, logistics, and detailed pilot statistics.',
    });
  });

  it('describes a shared log without its query', () => {
    const head = analysisHead(
      'EVE Log Analyzer',
      'Nicolas Kion — Combat Analysis',
      '1,983 damage dealt / 1,507 received — 14 DPS avg — 2m 20s combat',
      url,
    );
    expect(head.title).toBe('Nicolas Kion — Combat Analysis - EVE Log Analyzer');
    expect(tags(head)['og:url']).toBe('https://loganalyzer.eve-telescope.com/logs/abc');
    expect(tags(head)['og:description']).toBe(
      '1,983 damage dealt / 1,507 received — 14 DPS avg — 2m 20s combat',
    );
  });
});
