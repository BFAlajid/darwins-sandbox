import type { PrevalencePoint, SthStats } from './sth-types';

/** The engine emits prevalence as 0-1 fractions; history points are stored in percent. */
export function toPrevalencePoint(
  stats: Pick<SthStats, 'day' | 'prevalenceAscaris' | 'prevalenceTrichuris' | 'prevalenceHookworm' | 'prevalenceAny'>,
): PrevalencePoint {
  return {
    day: stats.day,
    ascaris: stats.prevalenceAscaris * 100,
    trichuris: stats.prevalenceTrichuris * 100,
    hookworm: stats.prevalenceHookworm * 100,
    any: stats.prevalenceAny * 100,
  };
}
