'use client';

import { useSthStore } from '@/stores/sth-store';
import { FLOATS_PER_AGENT, AgentType, agentTypeColor } from '@/lib/sth-types';

interface Props {
  agentBufferRef: React.RefObject<Float32Array | null>;
}

const AGENT_TYPE_NAMES: Record<number, string> = {
  [AgentType.Child]: 'Child',
  [AgentType.Parent]: 'Parent',
  [AgentType.Teacher]: 'Teacher',
  [AgentType.HealthWorker]: 'Health Worker',
};

function intensityLabel(value: number): { text: string; color: string } {
  if (value <= 0) return { text: 'Negative', color: 'text-green-400' };
  if (value <= 1) return { text: 'Light', color: 'text-yellow-400' };
  if (value <= 2) return { text: 'Moderate', color: 'text-orange-400' };
  return { text: 'Heavy', color: 'text-red-400' };
}

function kapLevelLabel(value: number): string {
  if (value >= 0.8) return 'High';
  if (value >= 0.5) return 'Medium';
  if (value >= 0.2) return 'Low';
  return 'Very Low';
}

export function AgentInspector({ agentBufferRef }: Props) {
  const selectedIdx = useSthStore((s) => s.selectedAgent);

  if (selectedIdx === null) {
    return (
      <div className="p-3 text-gray-500 text-xs italic">
        Click an agent to inspect
      </div>
    );
  }

  const data = agentBufferRef.current;
  if (!data) return null;

  const offset = selectedIdx * FLOATS_PER_AGENT;
  if (offset + FLOATS_PER_AGENT > data.length) {
    return (
      <div className="p-3 text-gray-500 text-xs italic">
        Agent no longer exists
      </div>
    );
  }

  // Read agent buffer fields
  // [x, y, target_x, target_y, agent_type, infection_status, epg_norm,
  //  knowledge, attitude, practice, household_id, school_id, r, g, b, barangay_id]
  const x = data[offset];
  const y = data[offset + 1];
  const agentType = data[offset + 4];
  const infection = data[offset + 5];
  const epgNorm = data[offset + 6];
  const knowledge = data[offset + 7];
  const attitude = data[offset + 8];
  const practice = data[offset + 9];
  const householdId = data[offset + 10];
  const schoolId = data[offset + 11];
  const r = data[offset + 12];
  const g = data[offset + 13];
  const b = data[offset + 14];

  const typeName = AGENT_TYPE_NAMES[agentType] ?? 'Unknown';
  const typeColor = agentTypeColor(agentType as AgentType);
  const cssColor = `rgb(${(r * 255) | 0}, ${(g * 255) | 0}, ${(b * 255) | 0})`;
  const infLabel = intensityLabel(infection);

  return (
    <div className="p-2">
      {/* Header */}
      <div className="flex items-center gap-2 mb-2">
        <div
          className="w-4 h-4 rounded-full"
          style={{ backgroundColor: typeColor }}
        />
        <span className="text-gray-200 text-xs font-bold">
          {typeName} #{selectedIdx}
        </span>
        <button
          onClick={() => useSthStore.getState().selectAgent(null)}
          className="ml-auto text-gray-500 hover:text-gray-300 text-xs"
        >
          x
        </button>
      </div>

      {/* Position */}
      <div className="flex justify-between text-xs mb-1">
        <span className="text-gray-500">Position</span>
        <span className="text-gray-300 font-mono">
          ({x.toFixed(0)}, {y.toFixed(0)})
        </span>
      </div>

      {/* Infection Status */}
      <div className="mb-2">
        <div className="flex justify-between text-xs mb-0.5">
          <span className="text-gray-500">Infection</span>
          <span className={`font-mono ${infLabel.color}`}>{infLabel.text}</span>
        </div>
        <div className="flex justify-between text-xs">
          <span className="text-gray-500">EPG (norm)</span>
          <span className="text-gray-300 font-mono">{(epgNorm * 100).toFixed(0)}%</span>
        </div>
      </div>

      {/* KAP Scores */}
      <div className="mb-2">
        <span className="text-gray-500 text-[10px] uppercase tracking-wider">
          KAP Scores
        </span>
        <KapBar label="Knowledge" value={knowledge} />
        <KapBar label="Attitude" value={attitude} />
        <KapBar label="Practice" value={practice} />
      </div>

      {/* IDs */}
      <div className="flex flex-col gap-0.5 mb-2">
        <div className="flex justify-between text-xs">
          <span className="text-gray-500">Household</span>
          <span className="text-gray-300 font-mono">#{householdId.toFixed(0)}</span>
        </div>
        {agentType === AgentType.Child && (
          <div className="flex justify-between text-xs">
            <span className="text-gray-500">School</span>
            <span className="text-gray-300 font-mono">#{schoolId.toFixed(0)}</span>
          </div>
        )}
      </div>

      {/* Color swatch */}
      <div className="flex items-center gap-2 text-xs">
        <span className="text-gray-500">Color</span>
        <div className="w-4 h-4 rounded" style={{ backgroundColor: cssColor }} />
      </div>
    </div>
  );
}

function KapBar({ label, value }: { label: string; value: number }) {
  const clampedValue = Math.max(0, Math.min(1, value));
  const levelText = kapLevelLabel(clampedValue);

  let barColor: string;
  if (clampedValue >= 0.7) barColor = '#22C55E';
  else if (clampedValue >= 0.4) barColor = '#EAB308';
  else barColor = '#EF4444';

  return (
    <div className="mt-1">
      <div className="flex justify-between text-xs mb-0.5">
        <span className="text-gray-400">{label}</span>
        <span className="text-gray-500 text-[10px]">
          {(clampedValue * 100).toFixed(0)}% ({levelText})
        </span>
      </div>
      <div className="w-full h-1.5 bg-gray-700 rounded-full">
        <div
          className="h-full rounded-full transition-all"
          style={{ width: `${clampedValue * 100}%`, backgroundColor: barColor }}
        />
      </div>
    </div>
  );
}
