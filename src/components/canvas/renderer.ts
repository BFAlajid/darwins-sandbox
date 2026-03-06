import { FLOATS_PER_CREATURE } from '@/lib/types';

interface Camera {
  x: number;
  y: number;
  zoom: number;
}

// Minimum visual size for creatures (in world units) so they're always visible
const MIN_RENDER_SIZE = 5;

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
  ctx.lineWidth = 2 / camera.zoom;
  ctx.strokeRect(0, 0, worldWidth, worldHeight);

  // Grid lines for orientation
  ctx.strokeStyle = '#1a1a30';
  ctx.lineWidth = 0.5 / camera.zoom;
  const gridStep = 100;
  for (let gx = gridStep; gx < worldWidth; gx += gridStep) {
    ctx.beginPath();
    ctx.moveTo(gx, 0);
    ctx.lineTo(gx, worldHeight);
    ctx.stroke();
  }
  for (let gy = gridStep; gy < worldHeight; gy += gridStep) {
    ctx.beginPath();
    ctx.moveTo(0, gy);
    ctx.lineTo(worldWidth, gy);
    ctx.stroke();
  }

  // --- Render food ---
  if (foodData && foodData.length > 0) {
    ctx.fillStyle = '#44cc44';
    const foodPath = new Path2D();
    const foodSize = Math.max(3, 4 / camera.zoom);
    for (let i = 0; i < foodData.length; i += 3) {
      const fx = foodData[i];
      const fy = foodData[i + 1];
      foodPath.rect(fx - foodSize * 0.5, fy - foodSize * 0.5, foodSize, foodSize);
    }
    ctx.fill(foodPath);
  }

  // --- Render creatures (pre-sorted by species_id) ---
  if (!creatureData || creatureData.length === 0) return;

  const creatureCount = creatureData.length / FLOATS_PER_CREATURE;

  // Effective pixel size of 1 world unit on screen
  const worldUnitPx = camera.zoom;

  // LOD based on effective screen size of a typical creature (~6 world units)
  // At zoom 1.0, a size-6 creature is 6px → LOD 1 (triangle)
  // We want triangles at almost all zoom levels since dots are hard to see
  const lod = worldUnitPx < 0.15 ? 0    // extreme zoom-out: dots
    : worldUnitPx < 0.5 ? 1              // far: simple triangles
    : worldUnitPx < 2.0 ? 2              // normal: detailed triangles
    : 3;                                  // close: full detail

  let currentSpecies = -1;
  let path = new Path2D();
  let currentColor = '';

  for (let i = 0; i < creatureCount; i++) {
    const offset = i * FLOATS_PER_CREATURE;
    const x = creatureData[offset];
    const y = creatureData[offset + 1];
    const rotation = creatureData[offset + 2];
    const rawSize = creatureData[offset + 3];
    const energyNorm = creatureData[offset + 4];
    const r = creatureData[offset + 5];
    const g = creatureData[offset + 6];
    const b = creatureData[offset + 7];
    const speciesId = creatureData[offset + 11];

    // Ensure creatures are always visible
    const size = Math.max(rawSize, MIN_RENDER_SIZE);

    // Batch by species — start new path when species changes
    if (speciesId !== currentSpecies) {
      if (currentColor) {
        ctx.fillStyle = currentColor;
        ctx.fill(path);
      }
      path = new Path2D();
      currentSpecies = speciesId;
      // Convert 0-1 RGB to CSS color with energy-based brightness
      const bright = 0.5 + energyNorm * 0.5;
      currentColor = `rgb(${(r * bright * 255) | 0}, ${(g * bright * 255) | 0}, ${(b * bright * 255) | 0})`;
    }

    if (lod === 0) {
      // Dot (only at extreme zoom-out)
      path.rect(x - 2, y - 2, 4, 4);
    } else if (lod === 1) {
      // Simple triangle
      drawTriangle(path, x, y, rotation, size * 0.8);
    } else {
      // Detailed triangle with size variation
      drawTriangle(path, x, y, rotation, size);

      // Energy bar at LOD 2+
      if (lod >= 2) {
        const barWidth = size * 2;
        const barHeight = Math.max(1.5, size * 0.2);
        const barY = y + size + 3;
        ctx.fillStyle = '#222';
        ctx.fillRect(x - barWidth * 0.5, barY, barWidth, barHeight);
        ctx.fillStyle = energyNorm > 0.5 ? '#4c4' : energyNorm > 0.2 ? '#cc4' : '#c44';
        ctx.fillRect(x - barWidth * 0.5, barY, barWidth * energyNorm, barHeight);
        ctx.fillStyle = currentColor;
      }

      // Eyes at LOD 3
      if (lod === 3) {
        const cos = Math.cos(rotation);
        const sin = Math.sin(rotation);
        const eyeOffset = size * 0.3;
        const eyeForward = size * 0.5;
        const eyeR = Math.max(1, size * 0.15);

        // Left eye
        const lex = x + cos * eyeForward + (-sin) * eyeOffset;
        const ley = y + sin * eyeForward + cos * eyeOffset;
        // Right eye
        const rex = x + cos * eyeForward - (-sin) * eyeOffset;
        const rey = y + sin * eyeForward - cos * eyeOffset;

        ctx.fillStyle = '#fff';
        ctx.beginPath();
        ctx.arc(lex, ley, eyeR, 0, Math.PI * 2);
        ctx.arc(rex, rey, eyeR, 0, Math.PI * 2);
        ctx.fill();

        ctx.fillStyle = '#000';
        ctx.beginPath();
        ctx.arc(lex + cos * eyeR * 0.3, ley + sin * eyeR * 0.3, eyeR * 0.5, 0, Math.PI * 2);
        ctx.arc(rex + cos * eyeR * 0.3, rey + sin * eyeR * 0.3, eyeR * 0.5, 0, Math.PI * 2);
        ctx.fill();

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
