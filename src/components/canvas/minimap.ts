import { FLOATS_PER_AGENT, FLOATS_PER_FACILITY, FacilityType } from '@/lib/sth-types';

const MINIMAP_WIDTH = 160;
const MINIMAP_HEIGHT = 120;
const MINIMAP_PADDING = 10;
const MINIMAP_BG = 'rgba(10, 10, 26, 0.85)';
const MINIMAP_BORDER = '#333355';
const VIEWPORT_COLOR = '#ffffff';

// Facility colors for minimap dots
const FACILITY_COLORS: Record<number, string> = {
  [FacilityType.Latrine]: '#8B5E3C',
  [FacilityType.WaterPump]: '#3B82F6',
  [FacilityType.HandwashStation]: '#22C55E',
  [FacilityType.School]: '#6B7280',
  [FacilityType.HealthCenter]: '#E5E7EB',
};

interface Camera {
  x: number;
  y: number;
  zoom: number;
}

/**
 * Renders a minimap overlay showing agent positions, facility locations,
 * and the current camera viewport.
 */
export function renderMinimap(
  ctx: CanvasRenderingContext2D,
  agentData: Float32Array | null,
  facilityData: Float32Array | null,
  agentCount: number,
  camera: Camera,
  canvasWidth: number,
  canvasHeight: number,
  worldWidth: number,
  worldHeight: number,
): void {
  if (worldWidth <= 0 || worldHeight <= 0) return;

  // Compute minimap dimensions preserving world aspect ratio
  const worldAspect = worldWidth / worldHeight;
  let mmW = MINIMAP_WIDTH;
  let mmH = MINIMAP_HEIGHT;

  if (worldAspect > mmW / mmH) {
    mmH = Math.round(mmW / worldAspect);
  } else {
    mmW = Math.round(mmH * worldAspect);
  }

  // Position in screen space (bottom-right corner)
  const mmX = canvasWidth - mmW - MINIMAP_PADDING;
  const mmY = canvasHeight - mmH - MINIMAP_PADDING;

  const scaleX = mmW / worldWidth;
  const scaleY = mmH / worldHeight;

  ctx.save();
  ctx.resetTransform();
  const dpr = window.devicePixelRatio || 1;
  ctx.scale(dpr, dpr);

  // Background
  ctx.fillStyle = MINIMAP_BG;
  ctx.fillRect(mmX, mmY, mmW, mmH);

  // Border
  ctx.strokeStyle = MINIMAP_BORDER;
  ctx.lineWidth = 1;
  ctx.strokeRect(mmX + 0.5, mmY + 0.5, mmW - 1, mmH - 1);

  // Facility dots (3px markers)
  if (facilityData) {
    const facilityCount = Math.floor(facilityData.length / FLOATS_PER_FACILITY);
    for (let i = 0; i < facilityCount; i++) {
      const offset = i * FLOATS_PER_FACILITY;
      const fx = facilityData[offset];
      const fy = facilityData[offset + 1];
      const fType = facilityData[offset + 2];

      const dotX = mmX + (fx / worldWidth) * mmW;
      const dotY = mmY + (fy / worldHeight) * mmH;

      ctx.fillStyle = FACILITY_COLORS[fType] ?? '#888';
      ctx.fillRect(dotX - 1.5, dotY - 1.5, 3, 3);
    }
  }

  // Agent dots (2px, colored by r/g/b from buffer)
  if (agentData) {
    const count = Math.min(agentCount, Math.floor(agentData.length / FLOATS_PER_AGENT));
    for (let i = 0; i < count; i++) {
      const offset = i * FLOATS_PER_AGENT;
      const ax = agentData[offset];
      const ay = agentData[offset + 1];
      const r = Math.round(agentData[offset + 12] * 255);
      const g = Math.round(agentData[offset + 13] * 255);
      const b = Math.round(agentData[offset + 14] * 255);

      const dotX = mmX + (ax / worldWidth) * mmW;
      const dotY = mmY + (ay / worldHeight) * mmH;

      ctx.fillStyle = `rgb(${r},${g},${b})`;
      ctx.fillRect(dotX - 1, dotY - 1, 2, 2);
    }
  }

  // Camera viewport rectangle
  const viewLeft = camera.x;
  const viewTop = camera.y;
  const viewWidth = canvasWidth / camera.zoom;
  const viewHeight = canvasHeight / camera.zoom;

  const rectX = Math.max(0, viewLeft) * scaleX + mmX;
  const rectY = Math.max(0, viewTop) * scaleY + mmY;
  const rectRight = Math.min(worldWidth, viewLeft + viewWidth) * scaleX + mmX;
  const rectBottom = Math.min(worldHeight, viewTop + viewHeight) * scaleY + mmY;
  const rectW = rectRight - rectX;
  const rectH = rectBottom - rectY;

  if (rectW > 0 && rectH > 0) {
    ctx.strokeStyle = VIEWPORT_COLOR;
    ctx.lineWidth = 1;
    ctx.strokeRect(
      Math.round(rectX) + 0.5,
      Math.round(rectY) + 0.5,
      Math.round(rectW),
      Math.round(rectH),
    );
  }

  ctx.restore();
}
