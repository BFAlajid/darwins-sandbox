import {
  FLOATS_PER_AGENT,
  FLOATS_PER_FACILITY,
  AgentType,
  FacilityType,
  type SthDataLens,
} from '@/lib/sth-types';

interface Camera {
  x: number;
  y: number;
  zoom: number;
}

// Barangay zone colors (semi-transparent for background fill)
const ZONE_COLORS: Record<string, string> = {
  residential: 'rgba(120, 100, 80, 0.15)',
  school: 'rgba(255, 200, 60, 0.12)',
  health_center: 'rgba(200, 60, 60, 0.12)',
  community: 'rgba(100, 180, 100, 0.12)',
  creek: 'rgba(60, 120, 200, 0.2)',
};

// Agent type sizes (world units radius)
const AGENT_SIZE: Record<number, number> = {
  [AgentType.Child]: 4,
  [AgentType.Parent]: 6,
  [AgentType.Teacher]: 6,
  [AgentType.HealthWorker]: 7,
};

// Agent type colors (used when dataLens === 'agents')
const AGENT_TYPE_COLORS: Record<number, string> = {
  [AgentType.Child]: '#4DB8E0',
  [AgentType.Parent]: '#80CC80',
  [AgentType.Teacher]: '#E6B333',
  [AgentType.HealthWorker]: '#E64D4D',
};

// Agent type label letters
const AGENT_TYPE_LABEL: Record<number, string> = {
  [AgentType.Child]: 'C',
  [AgentType.Parent]: 'P',
  [AgentType.Teacher]: 'T',
  [AgentType.HealthWorker]: 'H',
};

function infectionLensColor(intensity: number): string {
  if (intensity <= 0) return '#4CAF50'; // green - negative
  if (intensity <= 1) return '#FFC107'; // yellow - light
  if (intensity <= 2) return '#FF9800'; // orange - moderate
  return '#F44336'; // red - heavy
}

function kapLensColor(knowledge: number): string {
  // 0 = low (red) -> 1 = high (green)
  const r = ((1 - knowledge) * 220) | 0;
  const g = (knowledge * 200) | 0;
  return `rgb(${r}, ${g}, 60)`;
}

/** Day/night cycle tint. Returns alpha for a blue overlay (0 = day, up to 0.3 = night).
 *  Uses simulation tick (1 tick = 1 hour, 24 ticks = 1 day).
 */
function dayNightAlpha(tick: number): number {
  const hour = tick % 24; // 0-23
  // Night: 20:00-05:00, Dawn/Dusk transitions, Day: 06:00-19:00
  // Map hour to a sun height: peak at 12, trough at 0/24
  const angle = ((hour - 6) / 24) * Math.PI * 2; // 6am = 0, noon = π/2
  const sunHeight = Math.sin(angle); // -1 at midnight, +1 at noon
  if (sunHeight >= 0) return 0;
  return Math.min(0.3, Math.abs(sunHeight) * 0.35);
}

/**
 * Renders the full STH simulation frame onto a 2D canvas context.
 * Pure function — no React dependencies.
 *
 * @param viewportX — horizontal pixel offset for the viewport origin
 *                    (used in comparison mode to render into the right half)
 */
export function renderSthFrame(
  ctx: CanvasRenderingContext2D,
  agentBuffer: Float32Array | null,
  envBuffer: Float32Array | null,
  facilityBuffer: Float32Array | null,
  agentCount: number,
  envGridWidth: number,
  envGridHeight: number,
  envCellSize: number,
  camera: Camera,
  canvasWidth: number,
  canvasHeight: number,
  worldWidth: number,
  worldHeight: number,
  showContamination: boolean,
  showFacilities: boolean,
  dataLens: SthDataLens,
  selectedAgent: number | null,
  currentTick: number,
  viewportX: number = 0,
  showPaths: boolean = false,
  trailHistory: Float32Array[] | null = null,
): void {
  const dpr = window.devicePixelRatio || 1;

  // Clear viewport area
  ctx.resetTransform();
  ctx.scale(dpr, dpr);
  ctx.fillStyle = '#0d1117';
  ctx.fillRect(viewportX, 0, canvasWidth, canvasHeight);

  // Clip to viewport (for split-view comparison mode)
  ctx.save();
  ctx.beginPath();
  ctx.rect(viewportX, 0, canvasWidth, canvasHeight);
  ctx.clip();

  // Apply camera transform with viewport offset
  ctx.setTransform(
    camera.zoom * dpr, 0,
    0, camera.zoom * dpr,
    (viewportX - camera.x * camera.zoom) * dpr,
    -camera.y * camera.zoom * dpr,
  );

  // World boundary
  ctx.strokeStyle = '#30363d';
  ctx.lineWidth = 2 / camera.zoom;
  ctx.strokeRect(0, 0, worldWidth, worldHeight);

  // World background
  ctx.fillStyle = '#161b22';
  ctx.fillRect(0, 0, worldWidth, worldHeight);

  // Draw barangay zones (hardcoded layout — a real sim would send zone data)
  drawBarangayZones(ctx, worldWidth, worldHeight);

  // Contamination heatmap overlay
  if (showContamination && envBuffer && envGridWidth > 0 && envGridHeight > 0) {
    drawContaminationHeatmap(ctx, envBuffer, envGridWidth, envGridHeight, envCellSize);
  }

  // Facilities
  if (showFacilities && facilityBuffer && facilityBuffer.length >= FLOATS_PER_FACILITY) {
    drawFacilities(ctx, facilityBuffer, camera.zoom);
  }

  // Agent path trails (drawn under agents)
  if (showPaths && trailHistory && trailHistory.length > 0 && agentCount > 0) {
    drawAgentTrails(ctx, trailHistory, agentCount, camera.zoom);
  }

  // Agents
  if (agentBuffer && agentCount > 0) {
    drawAgents(ctx, agentBuffer, agentCount, camera.zoom, dataLens, selectedAgent);
  }

  // Day/night tint
  const nightAlpha = dayNightAlpha(currentTick);
  if (nightAlpha > 0.01) {
    ctx.fillStyle = `rgba(20, 30, 60, ${nightAlpha})`;
    ctx.fillRect(0, 0, worldWidth, worldHeight);
  }

  // Restore clip state (for split-view comparison mode)
  ctx.restore();
}

function drawBarangayZones(
  ctx: CanvasRenderingContext2D,
  worldWidth: number,
  worldHeight: number,
): void {
  const w = worldWidth;
  const h = worldHeight;

  // Simple 5-zone layout
  const zones = [
    { name: 'residential', x: 0, y: 0, w: w * 0.4, h: h * 0.5 },
    { name: 'school', x: w * 0.4, y: 0, w: w * 0.3, h: h * 0.4 },
    { name: 'health_center', x: w * 0.7, y: 0, w: w * 0.3, h: h * 0.4 },
    { name: 'community', x: 0, y: h * 0.5, w: w * 0.6, h: h * 0.5 },
    { name: 'creek', x: w * 0.6, y: h * 0.4, w: w * 0.4, h: h * 0.6 },
  ];

  for (const zone of zones) {
    ctx.fillStyle = ZONE_COLORS[zone.name] ?? 'rgba(100,100,100,0.1)';
    ctx.fillRect(zone.x, zone.y, zone.w, zone.h);

    // Zone label
    ctx.fillStyle = 'rgba(200, 200, 200, 0.2)';
    const fontSize = Math.max(10, Math.min(20, zone.w * 0.05));
    ctx.font = `${fontSize}px sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(zone.name.replace('_', ' '), zone.x + zone.w / 2, zone.y + zone.h / 2);
  }
}

function drawContaminationHeatmap(
  ctx: CanvasRenderingContext2D,
  envBuffer: Float32Array,
  gridWidth: number,
  gridHeight: number,
  cellSize: number,
): void {
  for (let gy = 0; gy < gridHeight; gy++) {
    for (let gx = 0; gx < gridWidth; gx++) {
      const idx = gy * gridWidth + gx;
      if (idx >= envBuffer.length) break;
      const value = envBuffer[idx];
      if (value < 0.01) continue;

      const alpha = Math.min(0.6, value * 0.6);
      ctx.fillStyle = `rgba(220, 50, 50, ${alpha})`;
      ctx.fillRect(gx * cellSize, gy * cellSize, cellSize, cellSize);
    }
  }
}

function drawFacilities(
  ctx: CanvasRenderingContext2D,
  facilityBuffer: Float32Array,
  zoom: number,
): void {
  const count = facilityBuffer.length / FLOATS_PER_FACILITY;
  const fontSize = Math.max(8, 12 / zoom);

  for (let i = 0; i < count; i++) {
    const offset = i * FLOATS_PER_FACILITY;
    const x = facilityBuffer[offset];
    const y = facilityBuffer[offset + 1];
    const type = facilityBuffer[offset + 2];
    const active = facilityBuffer[offset + 3];

    const opacity = active > 0.5 ? 1.0 : 0.4;

    ctx.save();
    ctx.globalAlpha = opacity;
    ctx.font = `bold ${fontSize}px monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    switch (type) {
      case FacilityType.Latrine: {
        // Brown square
        const s = 10;
        ctx.fillStyle = '#8B5E3C';
        ctx.fillRect(x - s / 2, y - s / 2, s, s);
        ctx.fillStyle = '#fff';
        ctx.fillText('T', x, y + 1);
        break;
      }
      case FacilityType.WaterPump: {
        // Blue circle
        ctx.fillStyle = '#3B82F6';
        ctx.beginPath();
        ctx.arc(x, y, 6, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = '#fff';
        ctx.fillText('W', x, y + 1);
        break;
      }
      case FacilityType.HandwashStation: {
        // Green circle
        ctx.fillStyle = '#22C55E';
        ctx.beginPath();
        ctx.arc(x, y, 6, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = '#fff';
        ctx.fillText('H', x, y + 1);
        break;
      }
      case FacilityType.School: {
        // Large grey rectangle
        const sw = 20;
        const sh = 16;
        ctx.fillStyle = '#6B7280';
        ctx.fillRect(x - sw / 2, y - sh / 2, sw, sh);
        ctx.fillStyle = '#fff';
        ctx.fillText('S', x, y + 1);
        break;
      }
      case FacilityType.HealthCenter: {
        // White rectangle with red cross
        const hw = 18;
        const hh = 14;
        ctx.fillStyle = '#E5E7EB';
        ctx.fillRect(x - hw / 2, y - hh / 2, hw, hh);
        ctx.fillStyle = '#DC2626';
        ctx.fillText('+', x, y + 1);
        break;
      }
    }

    ctx.restore();
  }
}

function drawAgents(
  ctx: CanvasRenderingContext2D,
  agentBuffer: Float32Array,
  agentCount: number,
  zoom: number,
  dataLens: SthDataLens,
  selectedAgent: number | null,
): void {
  // LOD based on zoom level
  const worldUnitPx = zoom;
  const lod = worldUnitPx < 0.3 ? 0 : worldUnitPx < 1.0 ? 1 : worldUnitPx < 3.0 ? 2 : 3;

  for (let i = 0; i < agentCount; i++) {
    const offset = i * FLOATS_PER_AGENT;
    if (offset + FLOATS_PER_AGENT > agentBuffer.length) break;

    const x = agentBuffer[offset];
    const y = agentBuffer[offset + 1];
    const agentType = agentBuffer[offset + 4];
    const infection = agentBuffer[offset + 5];
    const knowledge = agentBuffer[offset + 7];
    const r = agentBuffer[offset + 12];
    const g = agentBuffer[offset + 13];
    const b = agentBuffer[offset + 14];

    const size = AGENT_SIZE[agentType] ?? 5;

    // Determine color based on data lens
    let color: string;
    switch (dataLens) {
      case 'infection':
        color = infectionLensColor(infection);
        break;
      case 'kap':
        color = kapLensColor(knowledge);
        break;
      case 'agents':
        color = AGENT_TYPE_COLORS[agentType] ?? `rgb(${(r * 255) | 0}, ${(g * 255) | 0}, ${(b * 255) | 0})`;
        break;
      case 'contamination':
        // In contamination lens, agents are dim gray
        color = 'rgba(150, 150, 150, 0.5)';
        break;
      default:
        color = `rgb(${(r * 255) | 0}, ${(g * 255) | 0}, ${(b * 255) | 0})`;
    }

    if (lod === 0) {
      // Dots at extreme zoom-out
      ctx.fillStyle = color;
      ctx.fillRect(x - 1.5, y - 1.5, 3, 3);
    } else {
      // Circle
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(x, y, size, 0, Math.PI * 2);
      ctx.fill();

      // Type indicator at medium+ zoom
      if (lod >= 2) {
        const label = AGENT_TYPE_LABEL[agentType] ?? '?';
        ctx.fillStyle = '#fff';
        const fontSize = Math.max(6, size * 1.2);
        ctx.font = `bold ${fontSize}px monospace`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(label, x, y + 0.5);
      }

      // Infection indicator ring at LOD 3
      if (lod >= 3 && infection > 0) {
        ctx.strokeStyle = infectionLensColor(infection);
        ctx.lineWidth = 1.5 / zoom;
        ctx.beginPath();
        ctx.arc(x, y, size + 2, 0, Math.PI * 2);
        ctx.stroke();
      }
    }

    // Selected agent highlight
    if (selectedAgent === i) {
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 2 / zoom;
      ctx.beginPath();
      ctx.arc(x, y, size + 4, 0, Math.PI * 2);
      ctx.stroke();
    }
  }
}

/**
 * Draw fading trail lines for agent movement.
 * trailHistory is an array of snapshots, oldest first. Each snapshot is a
 * flat Float32Array of [x, y, x, y, ...] for all agents (2 floats each).
 */
function drawAgentTrails(
  ctx: CanvasRenderingContext2D,
  trailHistory: Float32Array[],
  agentCount: number,
  zoom: number,
): void {
  const segments = trailHistory.length;
  if (segments < 2) return;

  ctx.lineWidth = 1.5 / zoom;
  ctx.lineCap = 'round';

  for (let i = 0; i < agentCount; i++) {
    const off = i * 2;
    // Check that the last snapshot has data for this agent
    if (off + 1 >= trailHistory[segments - 1].length) break;

    ctx.beginPath();
    let started = false;

    for (let s = 0; s < segments; s++) {
      const snap = trailHistory[s];
      if (off + 1 >= snap.length) continue;
      const px = snap[off];
      const py = snap[off + 1];
      if (px === 0 && py === 0) continue; // skip uninitialized

      if (!started) {
        ctx.moveTo(px, py);
        started = true;
      } else {
        ctx.lineTo(px, py);
      }
    }

    if (started) {
      // Fade: older segments are more transparent
      const alpha = 0.25;
      ctx.strokeStyle = `rgba(180, 200, 255, ${alpha})`;
      ctx.stroke();
    }
  }
}
