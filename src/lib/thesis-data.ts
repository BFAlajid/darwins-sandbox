// Thesis Reference Data
// Source: STH prevalence, KAP levels, and EPG data from the Cebu study
// Used for calibration overlays and scenario presets

export const THESIS_DATA = {
  // Prevalence (Table 6) — percentages
  prevalence: {
    overall: 11.7,
    urban: 20.3,
    rural: 4.9,
  },

  // Species-specific prevalence (Table 7) — percentages
  speciesPrevalence: {
    ascaris: { overall: 11.7, urban: 20.3, rural: 4.9 },
    trichuris: { overall: 4.8, urban: 9.4, rural: 1.2 },
    hookworm: { overall: 0, urban: 0, rural: 0 },
  },

  // Mean EPG (Table 12)
  meanEpg: {
    ascaris: { urban: 10120.88, rural: 437.04 },
    trichuris: { urban: 118.5, rural: 1.19 },
  },

  // KAP levels — children (Tables 13, 16, 19) — percentages
  kapChildren: {
    knowledge: {
      urban: { poor: 44.5, moderate: 16.6, good: 14.7, excellent: 24.1 },
      rural: { poor: 32.8, moderate: 10.4, good: 31.3, excellent: 25.4 },
    },
    attitude: {
      urban: { negative: 11.9, neutral: 44.5, positive: 43.6 },
      rural: { negative: 6.0, neutral: 58.2, positive: 35.8 },
    },
    practice: {
      urban: { poor: 2.2, moderate: 72.7, good: 25.1 },
      rural: { poor: 0, moderate: 76.1, good: 23.9 },
    },
  },

  // Weighted mean KAP scores (0-100 scale, estimated from distribution)
  // Knowledge: poor=12.5, moderate=37.5, good=62.5, excellent=87.5
  // Attitude: negative=16.7, neutral=50, positive=83.3
  // Practice: poor=16.7, moderate=50, good=83.3
  kapMeanScores: {
    knowledge: {
      urban:
        0.445 * 12.5 + 0.166 * 37.5 + 0.147 * 62.5, // ~24.0 (+ 0.241 * 87.5 = ~45.0 total)
      rural:
        0.328 * 12.5 + 0.104 * 37.5 + 0.313 * 62.5, // ~23.6 (+ 0.254 * 87.5 = ~45.8 total)
    },
    attitude: {
      urban: 0.119 * 16.7 + 0.445 * 50 + 0.436 * 83.3, // ~60.6
      rural: 0.060 * 16.7 + 0.582 * 50 + 0.358 * 83.3, // ~59.9
    },
    practice: {
      urban: 0.022 * 16.7 + 0.727 * 50 + 0.251 * 83.3, // ~57.6
      rural: 0.0 * 16.7 + 0.761 * 50 + 0.239 * 83.3, // ~57.9
    },
  },

  // Study details
  sampleSize: { total: 145, urban: 64, rural: 81 },
  barangays: {
    urban: ['Guadalupe', 'Tisa'],
    rural: ['Sudlon II', 'Guba'],
  },
} as const;

// Computed weighted mean KAP scores for overlay markers (0-100 scale)
// Calculated from the KAP distribution percentages above
export const THESIS_KAP_MEANS = {
  knowledge:
    (0.445 * 12.5 + 0.166 * 37.5 + 0.147 * 62.5 + 0.241 * 87.5) * 0.44 +
    (0.328 * 12.5 + 0.104 * 37.5 + 0.313 * 62.5 + 0.254 * 87.5) * 0.56,
  // Weighted by sample proportion: urban=64/145=0.44, rural=81/145=0.56
  attitude:
    (0.119 * 16.7 + 0.445 * 50 + 0.436 * 83.3) * 0.44 +
    (0.060 * 16.7 + 0.582 * 50 + 0.358 * 83.3) * 0.56,
  practice:
    (0.022 * 16.7 + 0.727 * 50 + 0.251 * 83.3) * 0.44 +
    (0.0 * 16.7 + 0.761 * 50 + 0.239 * 83.3) * 0.56,
};

// Prevalence reference lines for the chart
export const THESIS_PREVALENCE_LINES = [
  {
    key: 'any' as const,
    value: THESIS_DATA.prevalence.overall,
    label: 'Thesis: Any STH',
  },
  {
    key: 'ascaris' as const,
    value: THESIS_DATA.speciesPrevalence.ascaris.overall,
    label: 'Thesis: Ascaris',
  },
  {
    key: 'trichuris' as const,
    value: THESIS_DATA.speciesPrevalence.trichuris.overall,
    label: 'Thesis: Trichuris',
  },
] as const;

// Scenario preset configurations (passed as JSON config strings to the worker)
export interface ScenarioPreset {
  id: string;
  name: string;
  description: string;
  color: string;
  config: Record<string, number | string>;
  /** Interventions to auto-apply after reset */
  autoInterventions?: Array<{
    type: string;
    delay: number; // days after start
    params: Record<string, number>;
  }>;
}

export const SCENARIO_PRESETS: ScenarioPreset[] = [
  {
    id: 'thesis-baseline',
    name: 'Thesis Baseline',
    description:
      'Parameters matching thesis conditions with biannual MDA (DOH protocol). Simulates the observed Cebu equilibrium at ~11.7% prevalence.',
    color: 'bg-gray-600',
    config: {
      auto_mda_interval_days: 180,
      auto_mda_drug: 0, // Albendazole
      auto_mda_coverage: 0.75, // WHO target
    },
  },
  {
    id: 'mda-only',
    name: 'MDA Only',
    description:
      'Biannual mass drug administration with Albendazole, no WASH improvements. Tests pharmacological-only control strategy.',
    color: 'bg-blue-700',
    config: {
      auto_mda_interval_days: 180,
      auto_mda_drug: 0, // Albendazole
      auto_mda_coverage: 0.85,
    },
  },
  {
    id: 'wash-only',
    name: 'WASH Only',
    description:
      'Build latrines + water pumps across barangays, no MDA. Tests infrastructure-only control strategy.',
    color: 'bg-amber-700',
    config: {
      auto_latrines: 10,
      auto_water_pumps: 5,
    },
  },
  {
    id: 'integrated',
    name: 'Integrated',
    description:
      'MDA + WASH + Education combined (thesis recommendation). The WHO-endorsed integrated approach.',
    color: 'bg-emerald-700',
    config: {
      auto_mda_interval_days: 180,
      auto_mda_drug: 0,
      auto_mda_coverage: 0.85,
      auto_latrines: 10,
      auto_water_pumps: 5,
      auto_education: 1,
    },
  },
  {
    id: 'covid-disruption',
    name: 'COVID Disruption',
    description:
      'School closures from months 6-18 disrupting MDA delivery. Models pandemic impact on deworming programs.',
    color: 'bg-red-700',
    config: {
      covid_school_closure_start: 6,
      covid_school_closure_end: 18,
      auto_mda_interval_days: 180,
      auto_mda_drug: 0,
      auto_mda_coverage: 0.75,
    },
  },
  {
    id: 'perfect-kap',
    name: 'What If: Perfect KAP',
    description:
      'All KAP scores maximized but structural factors unchanged. Demonstrates the thesis finding that knowledge alone is insufficient.',
    color: 'bg-purple-700',
    config: {
      initial_knowledge: 1.0,
      initial_attitude: 1.0,
      initial_practice: 1.0,
    },
  },
];
