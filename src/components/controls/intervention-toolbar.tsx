'use client';

import { useState } from 'react';
import { useSthStore } from '@/stores/sth-store';
import { FacilityType, DrugType, EducationMethod } from '@/lib/sth-types';

interface Props {
  onLaunchMDA: (schoolId: number, drug: number) => void;
  onBuildLatrine: (x: number, y: number) => void;
  onBuildWaterPump: (x: number, y: number) => void;
  onBuildHandwashStation: (x: number, y: number) => void;
  onLaunchEducation: (method: number, schoolId: number) => void;
  onLaunchBhwVisits: (coverage: number) => void;
  onIncreaseBudget: (multiplier: number) => void;
}

// Estimated costs (in PHP)
const COSTS = {
  mda: 500,
  latrine: 15000,
  waterPump: 25000,
  handwash: 8000,
  education: 3000,
  bhwVisit: 5000,
  budgetIncrease: 0,
};

export function InterventionToolbar({
  onLaunchMDA,
  onLaunchEducation,
  onLaunchBhwVisits,
  onIncreaseBudget,
}: Props) {
  const budget = useSthStore((s) => s.budgetRemaining);
  const facilityPlacementMode = useSthStore((s) => s.facilityPlacementMode);

  const [mdaDrug, setMdaDrug] = useState<DrugType>(DrugType.Albendazole);
  const [mdaSchool, setMdaSchool] = useState<number>(-1); // -1 = all schools
  const [eduMethod, setEduMethod] = useState<EducationMethod>(EducationMethod.Cartoon);
  const [bhwCoverage, setBhwCoverage] = useState<number>(0.8);

  const setPlacementMode = useSthStore.getState().setFacilityPlacementMode;

  const handleStartLatrine = () => {
    setPlacementMode(
      facilityPlacementMode === FacilityType.Latrine ? null : FacilityType.Latrine,
    );
  };

  const handleStartWaterPump = () => {
    setPlacementMode(
      facilityPlacementMode === FacilityType.WaterPump ? null : FacilityType.WaterPump,
    );
  };

  const handleStartHandwash = () => {
    setPlacementMode(
      facilityPlacementMode === FacilityType.HandwashStation
        ? null
        : FacilityType.HandwashStation,
    );
  };

  return (
    <div className="flex flex-wrap items-center gap-2 px-4 py-2 bg-gray-850 border-b border-gray-700 text-xs">
      {/* MDA Section */}
      <div className="flex items-center gap-1.5 border-r border-gray-700 pr-3">
        <span className="text-gray-500 font-medium">MDA:</span>
        <select
          value={mdaDrug}
          onChange={(e) => setMdaDrug(Number(e.target.value) as DrugType)}
          className="bg-gray-700 text-gray-300 rounded px-1.5 py-1 text-xs border-none outline-none"
        >
          <option value={DrugType.Albendazole}>Albendazole</option>
          <option value={DrugType.Mebendazole}>Mebendazole</option>
        </select>
        <select
          value={mdaSchool}
          onChange={(e) => setMdaSchool(Number(e.target.value))}
          className="bg-gray-700 text-gray-300 rounded px-1.5 py-1 text-xs border-none outline-none"
        >
          <option value={-1}>All Schools</option>
          <option value={0}>School 0</option>
          <option value={1}>School 1</option>
          <option value={2}>School 2</option>
        </select>
        <button
          onClick={() => onLaunchMDA(mdaSchool, mdaDrug)}
          disabled={budget < COSTS.mda}
          className="px-2 py-1 rounded bg-blue-700 text-white hover:bg-blue-600 disabled:opacity-40"
          title={`Cost: PHP ${COSTS.mda.toLocaleString()}`}
        >
          Launch MDA
        </button>
      </div>

      {/* WASH Section */}
      <div className="flex items-center gap-1.5 border-r border-gray-700 pr-3">
        <span className="text-gray-500 font-medium">WASH:</span>
        <button
          onClick={handleStartLatrine}
          disabled={budget < COSTS.latrine}
          className={`px-2 py-1 rounded text-white disabled:opacity-40 ${
            facilityPlacementMode === FacilityType.Latrine
              ? 'bg-amber-600 ring-1 ring-amber-400'
              : 'bg-amber-800 hover:bg-amber-700'
          }`}
          title={`Cost: PHP ${COSTS.latrine.toLocaleString()} — click map to place`}
        >
          Build Latrine
        </button>
        <button
          onClick={handleStartWaterPump}
          disabled={budget < COSTS.waterPump}
          className={`px-2 py-1 rounded text-white disabled:opacity-40 ${
            facilityPlacementMode === FacilityType.WaterPump
              ? 'bg-blue-600 ring-1 ring-blue-400'
              : 'bg-blue-800 hover:bg-blue-700'
          }`}
          title={`Cost: PHP ${COSTS.waterPump.toLocaleString()} — click map to place`}
        >
          Water Pump
        </button>
        <button
          onClick={handleStartHandwash}
          disabled={budget < COSTS.handwash}
          className={`px-2 py-1 rounded text-white disabled:opacity-40 ${
            facilityPlacementMode === FacilityType.HandwashStation
              ? 'bg-green-600 ring-1 ring-green-400'
              : 'bg-green-800 hover:bg-green-700'
          }`}
          title={`Cost: PHP ${COSTS.handwash.toLocaleString()} — click map to place`}
        >
          Handwash
        </button>
      </div>

      {/* Education Section */}
      <div className="flex items-center gap-1.5 border-r border-gray-700 pr-3">
        <span className="text-gray-500 font-medium">Edu:</span>
        <select
          value={eduMethod}
          onChange={(e) => setEduMethod(Number(e.target.value) as EducationMethod)}
          className="bg-gray-700 text-gray-300 rounded px-1.5 py-1 text-xs border-none outline-none"
        >
          <option value={EducationMethod.Cartoon}>Cartoon</option>
          <option value={EducationMethod.BoardGame}>Board Game</option>
          <option value={EducationMethod.TeacherLed}>Teacher-Led</option>
          <option value={EducationMethod.ParentMeeting}>Parent Meeting</option>
        </select>
        <button
          onClick={() => onLaunchEducation(eduMethod, mdaSchool)}
          disabled={budget < COSTS.education}
          className="px-2 py-1 rounded bg-purple-700 text-white hover:bg-purple-600 disabled:opacity-40"
          title={`Cost: PHP ${COSTS.education.toLocaleString()}`}
        >
          Launch
        </button>
      </div>

      {/* BHW Visits */}
      <div className="flex items-center gap-1.5 border-r border-gray-700 pr-3">
        <span className="text-gray-500 font-medium">BHW:</span>
        <input
          type="range"
          min={50}
          max={100}
          value={bhwCoverage * 100}
          onChange={(e) => setBhwCoverage(Number(e.target.value) / 100)}
          className="w-16 h-1 accent-teal-500"
          title={`Coverage: ${(bhwCoverage * 100).toFixed(0)}%`}
        />
        <span className="text-gray-400 w-8">{(bhwCoverage * 100).toFixed(0)}%</span>
        <button
          onClick={() => onLaunchBhwVisits(bhwCoverage)}
          disabled={budget < COSTS.bhwVisit}
          className="px-2 py-1 rounded bg-teal-700 text-white hover:bg-teal-600 disabled:opacity-40"
          title={`Cost: PHP ${COSTS.bhwVisit.toLocaleString()}`}
        >
          Deploy BHWs
        </button>
      </div>

      {/* Budget + Policy (right side) */}
      <div className="ml-auto flex items-center gap-3">
        <button
          onClick={() => onIncreaseBudget(1.5)}
          className="px-2 py-1 rounded bg-gray-700 text-gray-300 hover:bg-gray-600"
          title="Increase budget by 50%"
        >
          Increase Budget
        </button>
        <span className="text-gray-400 font-mono">
          Budget:{' '}
          <span className={budget < 5000 ? 'text-red-400' : 'text-emerald-400'}>
            PHP {budget.toLocaleString(undefined, { maximumFractionDigits: 0 })}
          </span>
        </span>
      </div>
    </div>
  );
}
