// STH Simulation Type Definitions
// Agent buffer layout: 16 floats per agent
// [x, y, target_x, target_y, agent_type, infection_status, epg_norm,
//  knowledge, attitude, practice, household_id, school_id, r, g, b, barangay_id]

export const FLOATS_PER_AGENT = 16;
export const FLOATS_PER_FACILITY = 4;

// Agent type indices (matches Rust enum)
export enum AgentType {
  Child = 0,
  Parent = 1,
  Teacher = 2,
  HealthWorker = 3,
}

// Infection intensity (WHO classification)
export enum InfectionIntensity {
  Negative = 0,
  Light = 1,
  Moderate = 2,
  Heavy = 3,
}

// Facility types
export enum FacilityType {
  Latrine = 0,
  WaterPump = 1,
  HandwashStation = 2,
  School = 3,
  HealthCenter = 4,
}

// Drug types
export enum DrugType {
  Albendazole = 0,
  Mebendazole = 1,
}

// Education methods
export enum EducationMethod {
  Cartoon = 0,
  BoardGame = 1,
  TeacherLed = 2,
  ParentMeeting = 3,
}

// Data lens options for visualization
export type SthDataLens = 'infection' | 'kap' | 'contamination' | 'agents';

// Simulation state
export type SimState = 'loading' | 'ready' | 'running' | 'paused' | 'error';

// Stats from WASM (JSON, throttled to 5Hz)
export interface SthStats {
  tick: number;
  day: number;
  month: number;
  agentCount: number;
  prevalenceAscaris: number;
  prevalenceTrichuris: number;
  prevalenceHookworm: number;
  prevalenceAny: number;
  meanEpg: number;
  meanKnowledge: number;
  meanAttitude: number;
  meanPractice: number;
  latrineCoverage: number;
  waterCoverage: number;
  budgetRemaining: number;
  budgetSpent: number;
  mdaRounds: number;
  childrenTreated: number;
  barangays: BarangayStatsJson[];
}

// Per-barangay breakdown for comparison mode
export interface BarangayStatsJson {
  name: string;
  setting: 'urban' | 'rural';
  prevalenceAny: number;
  prevalenceAscaris: number;
  prevalenceTrichuris: number;
  prevalenceHookworm: number;
  meanEpg: number;
  meanKnowledge: number;
  meanAttitude: number;
  meanPractice: number;
  latrineCoverage: number;
  waterCoverage: number;
}

// Agent detail (from clicking/inspecting an agent)
export interface AgentDetail {
  index: number;
  agentType: AgentType;
  age: number;
  householdId: number;
  schoolId: number;
  x: number;
  y: number;
  ascarisEpg: number;
  ascarisIntensity: string;
  trichurisEpg: number;
  trichurisIntensity: string;
  hookwormEpg: number;
  hookwormIntensity: string;
  knowledge: number;
  attitude: number;
  practice: number;
  wearsShoes: boolean;
  washesHands: boolean;
  usesLatrine: boolean;
  daysSinceDeworming: number;
  hasLatrine: boolean;
  hasWater: boolean;
  hasWashFacility: boolean;
}

// Events from simulation
export interface SthEvent {
  tick: number;
  eventType: string;
  message: string;
}

// Prevalence history point (for time-series chart)
export interface PrevalencePoint {
  day: number;
  ascaris: number;
  trichuris: number;
  hookworm: number;
  any: number;
}

// Intervention event log
export interface InterventionEvent {
  day: number;
  type: string;
  description: string;
  cost: number;
  result?: string;
}

// Worker commands (main thread -> worker)
export type SthCommand =
  | { type: 'init'; seed?: number; config?: string; comparisonMode?: boolean }
  | { type: 'step'; count: number }
  | { type: 'pause' }
  | { type: 'resume' }
  | { type: 'reset'; seed?: number; config?: string; comparisonMode?: boolean }
  | { type: 'toggleComparisonMode'; enabled: boolean }
  | { type: 'launchMDA'; schoolId: number; drug: number; barangayId?: number }
  | { type: 'buildLatrine'; x: number; y: number; barangayId?: number }
  | { type: 'buildWaterPump'; x: number; y: number; barangayId?: number }
  | { type: 'buildHandwashStation'; x: number; y: number; barangayId?: number }
  | { type: 'launchEducation'; method: number; schoolId: number; barangayId?: number }
  | { type: 'launchBhwVisits'; coverage: number; barangayId?: number }
  | { type: 'increaseBudget'; multiplier: number; barangayId?: number }
  | { type: 'selectAgent'; index: number | null }
  | { type: 'returnBuffer'; buffer: ArrayBuffer };

// Worker messages (worker -> main thread)
export type SthWorkerMessage =
  | { type: 'ready'; seed: number; worldWidth: number; worldHeight: number }
  | {
      type: 'frame';
      agentBuffer: ArrayBuffer;
      envBuffer: ArrayBuffer;
      facilityBuffer: ArrayBuffer;
      stats: string;
      events: string;
      agentCount: number;
      envGridWidth: number;
      envGridHeight: number;
      envCellSize: number;
      // Comparison mode: second simulation data (null when single mode)
      agentBuffer2?: ArrayBuffer;
      envBuffer2?: ArrayBuffer;
      facilityBuffer2?: ArrayBuffer;
      stats2?: string;
      events2?: string;
      agentCount2?: number;
      envGridWidth2?: number;
      envGridHeight2?: number;
      envCellSize2?: number;
    }
  | { type: 'error'; message: string }
  | { type: 'heartbeat' }
  | { type: 'agentDetail'; detail: string };

// Color helpers

/** Returns CSS color string for an agent type */
export function agentTypeColor(type: AgentType): string {
  switch (type) {
    case AgentType.Child:
      return '#4DB8E0';
    case AgentType.Parent:
      return '#80CC80';
    case AgentType.Teacher:
      return '#E6B333';
    case AgentType.HealthWorker:
      return '#E64D4D';
  }
}

/** Returns CSS color string for an infection intensity level */
export function infectionColor(intensity: InfectionIntensity): string {
  switch (intensity) {
    case InfectionIntensity.Negative:
      return '#4CAF50';
    case InfectionIntensity.Light:
      return '#FFC107';
    case InfectionIntensity.Moderate:
      return '#FF9800';
    case InfectionIntensity.Heavy:
      return '#F44336';
  }
}

/** Returns display icon for a facility type */
export function facilityIcon(type: FacilityType): string {
  switch (type) {
    case FacilityType.Latrine:
      return '\u{1F6BB}'; // restroom symbol
    case FacilityType.WaterPump:
      return '\u{1F4A7}'; // water drop
    case FacilityType.HandwashStation:
      return '\u{1F9FC}'; // soap
    case FacilityType.School:
      return '\u{1F3EB}'; // school
    case FacilityType.HealthCenter:
      return '\u{1F3E5}'; // hospital
  }
}
