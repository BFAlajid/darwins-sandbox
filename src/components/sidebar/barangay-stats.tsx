'use client';

import { useSthStore } from '@/stores/sth-store';
import type { SthStats } from '@/lib/sth-types';

export function BarangayStats() {
  const stats = useSthStore((s) => s.stats);
  const stats2 = useSthStore((s) => s.stats2);
  const comparisonMode = useSthStore((s) => s.comparisonMode);

  if (!stats) {
    return (
      <div className="p-3 text-gray-500 text-xs italic">
        Waiting for simulation data...
      </div>
    );
  }

  if (comparisonMode && stats2) {
    return <ComparisonStats urban={stats} rural={stats2} />;
  }

  return <SingleStats stats={stats} />;
}

/** Single-simulation stats view (existing behavior) */
function SingleStats({ stats }: { stats: SthStats }) {
  return (
    <div className="p-2 flex flex-col gap-2">
      {/* Overall Prevalence */}
      <div>
        <div className="flex justify-between text-xs mb-1">
          <span className="text-gray-400">Overall STH Prevalence</span>
          <span className={`font-mono font-bold ${prevalenceColor(stats.prevalenceAny)}`}>
            {(stats.prevalenceAny * 100).toFixed(1)}%
          </span>
        </div>
      </div>

      {/* Per-species Prevalence */}
      <div>
        <span className="text-gray-500 text-[10px] uppercase tracking-wider">
          By Species
        </span>
        <PrevalenceBar label="Ascaris" value={stats.prevalenceAscaris} color="#F59E0B" />
        <PrevalenceBar label="Trichuris" value={stats.prevalenceTrichuris} color="#8B5CF6" />
        <PrevalenceBar label="Hookworm" value={stats.prevalenceHookworm} color="#EF4444" />
      </div>

      {/* Mean KAP */}
      <div>
        <span className="text-gray-500 text-[10px] uppercase tracking-wider">
          Mean KAP Scores
        </span>
        <StatRow label="Knowledge" value={`${(stats.meanKnowledge * 100).toFixed(0)}%`} />
        <StatRow label="Attitude" value={`${(stats.meanAttitude * 100).toFixed(0)}%`} />
        <StatRow label="Practice" value={`${(stats.meanPractice * 100).toFixed(0)}%`} />
      </div>

      {/* WASH Coverage */}
      <div>
        <span className="text-gray-500 text-[10px] uppercase tracking-wider">
          WASH Coverage
        </span>
        <PrevalenceBar label="Latrine" value={stats.latrineCoverage} color="#D97706" />
        <PrevalenceBar label="Water" value={stats.waterCoverage} color="#3B82F6" />
      </div>

      {/* MDA Stats */}
      <div>
        <span className="text-gray-500 text-[10px] uppercase tracking-wider">
          MDA Program
        </span>
        <StatRow label="Rounds" value={String(stats.mdaRounds)} />
        <StatRow label="Children Treated" value={stats.childrenTreated.toLocaleString()} />
      </div>

      {/* Mean EPG */}
      <div className="flex justify-between text-xs border-t border-gray-700 pt-1 mt-1">
        <span className="text-gray-500">Mean EPG</span>
        <span className="text-gray-300 font-mono">{stats.meanEpg.toFixed(0)}</span>
      </div>
    </div>
  );
}

/** Comparison mode: side-by-side urban vs rural stats */
function ComparisonStats({ urban, rural }: { urban: SthStats; rural: SthStats }) {
  return (
    <div className="p-2 flex flex-col gap-2">
      {/* Header */}
      <div className="grid grid-cols-3 gap-1 text-[10px] uppercase tracking-wider font-bold border-b border-gray-700 pb-1">
        <span className="text-gray-500" />
        <span className="text-amber-400 text-center">Urban</span>
        <span className="text-emerald-400 text-center">Rural</span>
      </div>

      {/* Overall STH Prevalence */}
      <ComparisonRow
        label="STH Prevalence"
        urban={urban.prevalenceAny}
        rural={rural.prevalenceAny}
        format="percent"
        highlightHigher
      />

      {/* Per-species */}
      <div>
        <span className="text-gray-500 text-[10px] uppercase tracking-wider">
          By Species
        </span>
        <ComparisonBarPair
          label="Ascaris"
          urban={urban.prevalenceAscaris}
          rural={rural.prevalenceAscaris}
          color="#F59E0B"
        />
        <ComparisonBarPair
          label="Trichuris"
          urban={urban.prevalenceTrichuris}
          rural={rural.prevalenceTrichuris}
          color="#8B5CF6"
        />
        <ComparisonBarPair
          label="Hookworm"
          urban={urban.prevalenceHookworm}
          rural={rural.prevalenceHookworm}
          color="#EF4444"
        />
      </div>

      {/* Mean KAP */}
      <div>
        <span className="text-gray-500 text-[10px] uppercase tracking-wider">
          Mean KAP Scores
        </span>
        <ComparisonRow label="Knowledge" urban={urban.meanKnowledge} rural={rural.meanKnowledge} format="percent" />
        <ComparisonRow label="Attitude" urban={urban.meanAttitude} rural={rural.meanAttitude} format="percent" />
        <ComparisonRow label="Practice" urban={urban.meanPractice} rural={rural.meanPractice} format="percent" />
      </div>

      {/* WASH Coverage */}
      <div>
        <span className="text-gray-500 text-[10px] uppercase tracking-wider">
          WASH Coverage
        </span>
        <ComparisonRow label="Latrine" urban={urban.latrineCoverage} rural={rural.latrineCoverage} format="percent" />
        <ComparisonRow label="Water" urban={urban.waterCoverage} rural={rural.waterCoverage} format="percent" />
      </div>

      {/* Mean EPG */}
      <ComparisonRow
        label="Mean EPG"
        urban={urban.meanEpg}
        rural={rural.meanEpg}
        format="number"
        highlightHigher
      />

      {/* Agent counts */}
      <ComparisonRow
        label="Agents"
        urban={urban.agentCount}
        rural={rural.agentCount}
        format="number"
      />
    </div>
  );
}

/** A row showing urban vs rural values side by side */
function ComparisonRow({
  label,
  urban,
  rural,
  format,
  highlightHigher = false,
}: {
  label: string;
  urban: number;
  rural: number;
  format: 'percent' | 'number';
  highlightHigher?: boolean;
}) {
  const formatVal = (v: number) =>
    format === 'percent' ? `${(v * 100).toFixed(1)}%` : v.toFixed(0);

  const urbanHigher = urban > rural;
  const diff = Math.abs(urban - rural);
  const showDiff = highlightHigher && diff > 0.001;

  return (
    <div className="grid grid-cols-3 gap-1 text-xs mt-0.5">
      <span className="text-gray-400 truncate">{label}</span>
      <span
        className={`text-center font-mono ${
          highlightHigher && urbanHigher ? 'text-red-400 font-bold' : 'text-gray-300'
        }`}
      >
        {formatVal(urban)}
      </span>
      <span
        className={`text-center font-mono ${
          highlightHigher && !urbanHigher ? 'text-red-400 font-bold' : 'text-gray-300'
        }`}
      >
        {formatVal(rural)}
      </span>
    </div>
  );
}

/** Dual prevalence bars for comparison mode */
function ComparisonBarPair({
  label,
  urban,
  rural,
  color,
}: {
  label: string;
  urban: number;
  rural: number;
  color: string;
}) {
  const urbanClamped = Math.max(0, Math.min(1, urban));
  const ruralClamped = Math.max(0, Math.min(1, rural));

  return (
    <div className="mt-1">
      <div className="flex justify-between text-xs mb-0.5">
        <span className="text-gray-400">{label}</span>
        <span className="text-gray-500 font-mono text-[9px]">
          U:{(urbanClamped * 100).toFixed(1)}% / R:{(ruralClamped * 100).toFixed(1)}%
        </span>
      </div>
      {/* Urban bar */}
      <div className="w-full h-1 bg-gray-700 rounded-full mb-0.5">
        <div
          className="h-full rounded-full transition-all"
          style={{ width: `${urbanClamped * 100}%`, backgroundColor: color, opacity: 1 }}
        />
      </div>
      {/* Rural bar (slightly dimmer) */}
      <div className="w-full h-1 bg-gray-700 rounded-full">
        <div
          className="h-full rounded-full transition-all"
          style={{ width: `${ruralClamped * 100}%`, backgroundColor: color, opacity: 0.6 }}
        />
      </div>
    </div>
  );
}

function PrevalenceBar({
  label,
  value,
  color,
}: {
  label: string;
  value: number;
  color: string;
}) {
  const clamped = Math.max(0, Math.min(1, value));

  return (
    <div className="mt-1">
      <div className="flex justify-between text-xs mb-0.5">
        <span className="text-gray-400">{label}</span>
        <span className="text-gray-300 font-mono text-[10px]">
          {(clamped * 100).toFixed(1)}%
        </span>
      </div>
      <div className="w-full h-1.5 bg-gray-700 rounded-full">
        <div
          className="h-full rounded-full transition-all"
          style={{ width: `${clamped * 100}%`, backgroundColor: color }}
        />
      </div>
    </div>
  );
}

function StatRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between text-xs mt-0.5">
      <span className="text-gray-400">{label}</span>
      <span className="text-gray-300 font-mono">{value}</span>
    </div>
  );
}

function prevalenceColor(value: number): string {
  if (value >= 0.5) return 'text-red-400';
  if (value >= 0.2) return 'text-orange-400';
  if (value >= 0.05) return 'text-yellow-400';
  return 'text-green-400';
}
