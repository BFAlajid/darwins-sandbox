import { FLOATS_PER_CREATURE } from '@/lib/types';

interface Camera {
  x: number;
  y: number;
  zoom: number;
}

export function renderFrame(
  ctx: CanvasRenderingContext2D,
  creatureData: Float32Array | null,
  foodData: Float32Array | null,
  camera: Camera,
  canvasWidth: number,
  canvasHeight: number,
  worldWidth: number,
  worldHeight: number,
) {
  const dpr = window.devicePixelRatio || 1;

  // Clear
  ctx.resetTransform();
  ctx.scale(dpr, dpr);
  ctx.fillStyle = '#0a0a1a';
  ctx.fillRect(0, 0, canvasWidth, canvasHeight);

  // Apply camera transform
  ctx.setTransform(
    camera.zoom * dpr, 0,
    0, camera.zoom * dpr,
    -camera.x * camera.zoom * dpr,
    -camera.y * camera.zoom * dpr,
  );

  // World boundary
  ctx.strokeStyle = '#333355';
  ctx.lineWidth = 1 / camera.zoom;
  ctx.strokeRect(0, 0, worldWidth, worldHeight);

  // --- Render food ---
  if (foodData && foodData.length > 0) {
    ctx.fillStyle = '#44cc44';
    const foodPath = new Path2D();
    const foodSize = Math.max(1.5, 2 / camera.zoom);
    for (let i = 0; i < foodData.length; i += 3) {
      const fx = foodData[i];
      const fy = foodData[i + 1];
      foodPath.rect(
        (fx - foodSize * 0.5) | 0,
        (fy - foodSize * 0.5) | 0,
        foodSize,
        foodSize,
      );
    }
    ctx.fill(foodPath);
  }

  // --- Render creatures (pre-sorted by species_id) ---
  if (!creatureData || creatureData.length === 0) return;

  const creatureCount = creatureData.length / FLOATS_PER_CREATURE;

  // Determine LOD from zoom
  const lod = camera.zoom < 0.3 ? 0
    : camera.zoom < 0.7 ? 1
    : camera.zoom < 1.5 ? 2
    : 3;

  let currentSpecies = -1;
  let path = new Path2D();
  let currentColor = '';

  for (let i = 0; i < creatureCount; i++) {
    const offset = i * FLOATS_PER_CREATURE;
    const x = creatureData[offset];
    const y = creatureData[offset + 1];
    const rotation = creatureData[offset + 2];
    const size = creatureData[offset + 3];
    const energyNorm = creatureData[offset + 4];
    const r = creatureData[offset + 5];
    const g = creatureData[offset + 6];
    const b = creatureData[offset + 7];
    const speciesId = creatureData[offset + 11];

    // Batch by species — start new path when species changes
    if (speciesId !== currentSpecies) {
      if (currentColor) {
        ctx.fillStyle = currentColor;
        ctx.fill(path);
      }
      path = new Path2D();
      currentSpecies = speciesId;
      // Convert 0-1 RGB to CSS color with energy-based saturation
      const satMul = 0.4 + energyNorm * 0.6;
      currentColor = `rgb(${(r * satMul * 255) | 0}, ${(g * satMul * 255) | 0}, ${(b * satMul * 255) | 0})`;
    }

    if (lod === 0) {
      // Dot
      path.rect((x - 1) | 0, (y - 1) | 0, 2, 2);
    } else if (lod === 1) {
      // Simple triangle
      drawTriangle(path, x, y, rotation, size * 0.8);
    } else {
      // Detailed triangle with size variation
      drawTriangle(path, x, y, rotation, size);

      // Energy bar at LOD 2+
      if (lod >= 2) {
        // Small energy indicator below creature
        const barWidth = size * 1.5;
        const barHeight = 1.5;
        const barY = y + size + 2;
        // Background
        ctx.fillStyle = '#333';
        ctx.fillRect(x - barWidth * 0.5, barY, barWidth, barHeight);
        // Fill
        ctx.fillStyle = energyNorm > 0.5 ? '#4c4' : energyNorm > 0.2 ? '#cc4' : '#c44';
        ctx.fillRect(x - barWidth * 0.5, barY, barWidth * energyNorm, barHeight);
        // Reset fill to species color for next creature
        ctx.fillStyle = currentColor;
      }
    }
  }

  // Flush last batch
  if (currentColor) {
    ctx.fillStyle = currentColor;
    ctx.fill(path);
  }
}

function drawTriangle(path: Path2D, x: number, y: number, rotation: number, size: number) {
  const cos = Math.cos(rotation);
  const sin = Math.sin(rotation);
  const len = size * 1.2;
  const half = size * 0.5;

  // Nose
  const nx = x + cos * len;
  const ny = y + sin * len;
  // Left
  const lx = x + (-sin) * half - cos * half;
  const ly = y + cos * half - sin * half;
  // Right
  const rx = x - (-sin) * half - cos * half;
  const ry = y - cos * half - sin * half;

  path.moveTo(nx, ny);
  path.lineTo(lx, ly);
  path.lineTo(rx, ry);
  path.closePath();
}
