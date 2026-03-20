/**
 * Data export utilities for STH simulation.
 * Exports prevalence history, intervention log, and canvas screenshots.
 */

import type { PrevalencePoint, InterventionEvent, SthStats } from './sth-types';

/**
 * Export prevalence history as CSV.
 * Columns: Day, Ascaris%, Trichuris%, Hookworm%, Any%
 */
export function exportPrevalenceCsv(
  history: PrevalencePoint[],
  filename = 'sth-prevalence.csv',
): void {
  if (history.length === 0) return;

  const header = 'Day,Ascaris%,Trichuris%,Hookworm%,Any%';
  const rows = history.map(
    (p) => `${p.day},${p.ascaris.toFixed(2)},${p.trichuris.toFixed(2)},${p.hookworm.toFixed(2)},${p.any.toFixed(2)}`,
  );
  const csv = [header, ...rows].join('\n');
  downloadBlob(csv, filename, 'text/csv');
}

/**
 * Export comparison mode data: both urban and rural prevalence side-by-side.
 */
export function exportComparisonCsv(
  urban: PrevalencePoint[],
  rural: PrevalencePoint[],
  filename = 'sth-comparison.csv',
): void {
  const maxLen = Math.max(urban.length, rural.length);
  if (maxLen === 0) return;

  const header = 'Day,Urban_Ascaris%,Urban_Trichuris%,Urban_Hookworm%,Urban_Any%,Rural_Ascaris%,Rural_Trichuris%,Rural_Hookworm%,Rural_Any%';
  const rows: string[] = [];

  for (let i = 0; i < maxLen; i++) {
    const u = urban[i];
    const r = rural[i];
    const day = u?.day ?? r?.day ?? i;
    rows.push([
      day,
      u?.ascaris.toFixed(2) ?? '',
      u?.trichuris.toFixed(2) ?? '',
      u?.hookworm.toFixed(2) ?? '',
      u?.any.toFixed(2) ?? '',
      r?.ascaris.toFixed(2) ?? '',
      r?.trichuris.toFixed(2) ?? '',
      r?.hookworm.toFixed(2) ?? '',
      r?.any.toFixed(2) ?? '',
    ].join(','));
  }

  const csv = [header, ...rows].join('\n');
  downloadBlob(csv, filename, 'text/csv');
}

/**
 * Export intervention log as CSV.
 */
export function exportInterventionsCsv(
  log: InterventionEvent[],
  filename = 'sth-interventions.csv',
): void {
  if (log.length === 0) return;

  const header = 'Day,Type,Description,Cost';
  const rows = log.map(
    (e) => `${e.day},${sanitizeCsvField(e.type)},"${sanitizeCsvField(e.description).replace(/"/g, '""')}",${e.cost.toFixed(2)}`,
  );
  const csv = [header, ...rows].join('\n');
  downloadBlob(csv, filename, 'text/csv');
}

/** Prevent CSV formula injection by prefixing dangerous chars with a single quote. */
function sanitizeCsvField(value: string): string {
  if (/^[=+\-@\t\r]/.test(value)) {
    return "'" + value;
  }
  return value;
}

/**
 * Export current stats as a JSON snapshot.
 */
export function exportStatsJson(
  stats: SthStats,
  filename = 'sth-stats.json',
): void {
  const json = JSON.stringify(stats, null, 2);
  downloadBlob(json, filename, 'application/json');
}

/**
 * Capture a screenshot of the simulation canvas.
 */
export function captureScreenshot(
  canvas: HTMLCanvasElement,
  filename = 'sth-screenshot.png',
): void {
  canvas.toBlob((blob) => {
    if (!blob) return;
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }, 'image/png');
}

function downloadBlob(content: string, filename: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
