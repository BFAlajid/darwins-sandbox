'use client';

import { useCallback, useEffect, useRef } from 'react';
import { useSthStore } from '@/stores/sth-store';

const BG_COLOR = '#1a1a2e';
const GRID_COLOR = 'rgba(255, 255, 255, 0.08)';
const LABEL_COLOR = 'rgba(255, 255, 255, 0.5)';

const CANVAS_HEIGHT = 180;
const RENDER_INTERVAL_MS = 500;

// Category classification (matches intervention-timeline)
function interventionCategory(type: string): string {
  const t = type.toLowerCase();
  if (t.includes('mda') || t.includes('deworm') || t.includes('drug')) return 'MDA';
  if (t.includes('wash') || t.includes('latrine') || t.includes('water') || t.includes('handwash')) return 'WASH';
  if (t.includes('edu') || t.includes('teach') || t.includes('cartoon') || t.includes('board') || t.includes('game')) return 'Education';
  if (t.includes('bhw') || t.includes('health') || t.includes('visit')) return 'BHW';
  return 'Other';
}

const CATEGORIES: { key: string; color: string; fillColor: string }[] = [
  { key: 'MDA', color: 'rgba(239, 68, 68, 1)', fillColor: 'rgba(239, 68, 68, 0.7)' },
  { key: 'WASH', color: 'rgba(59, 130, 246, 1)', fillColor: 'rgba(59, 130, 246, 0.7)' },
  { key: 'Education', color: 'rgba(34, 197, 94, 1)', fillColor: 'rgba(34, 197, 94, 0.7)' },
  { key: 'BHW', color: 'rgba(168, 85, 247, 1)', fillColor: 'rgba(168, 85, 247, 0.7)' },
  { key: 'Other', color: 'rgba(220, 220, 220, 1)', fillColor: 'rgba(220, 220, 220, 0.7)' },
];

function formatPeso(amount: number): string {
  return '\u20B1' + amount.toLocaleString(undefined, { maximumFractionDigits: 0 });
}

export function CostTracker() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const drawChart = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = container.getBoundingClientRect();
    const width = rect.width;
    if (width <= 0) return;

    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(CANVAS_HEIGHT * dpr);
    canvas.style.width = `${width}px`;
    canvas.style.height = `${CANVAS_HEIGHT}px`;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    ctx.fillStyle = BG_COLOR;
    ctx.fillRect(0, 0, width, CANVAS_HEIGHT);

    const state = useSthStore.getState();
    const budgetRemaining = state.budgetRemaining;
    const budgetSpent = state.budgetSpent;
    const totalBudget = budgetRemaining + budgetSpent;
    const interventionLog = state.interventionLog;

    const padding = { top: 24, bottom: 8, left: 10, right: 10 };

    // Title
    ctx.font = '10px monospace';
    ctx.textBaseline = 'top';
    ctx.fillStyle = LABEL_COLOR;
    ctx.fillText('Budget Allocation', padding.left, 4);

    if (totalBudget <= 0) {
      ctx.font = '11px monospace';
      ctx.fillStyle = LABEL_COLOR;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('No budget data', width / 2, CANVAS_HEIGHT / 2);
      ctx.textAlign = 'start';
      return;
    }

    // --- Budget Bar ---
    const barY = padding.top + 4;
    const barH = 20;
    const barW = width - padding.left - padding.right;
    const spentFraction = Math.min(1, budgetSpent / totalBudget);

    // Background (remaining)
    ctx.fillStyle = 'rgba(255, 255, 255, 0.1)';
    ctx.beginPath();
    roundRect(ctx, padding.left, barY, barW, barH, 4);
    ctx.fill();

    // Spent portion
    if (spentFraction > 0) {
      const spentW = Math.max(2, spentFraction * barW);
      ctx.fillStyle = spentFraction > 0.9
        ? 'rgba(239, 68, 68, 0.6)'
        : spentFraction > 0.7
          ? 'rgba(249, 115, 22, 0.6)'
          : 'rgba(34, 197, 94, 0.6)';
      ctx.beginPath();
      roundRect(ctx, padding.left, barY, spentW, barH, 4);
      ctx.fill();
    }

    // Budget text
    ctx.font = '10px monospace';
    ctx.fillStyle = '#e5e7eb';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(
      `${formatPeso(budgetSpent)} / ${formatPeso(totalBudget)} (${(spentFraction * 100).toFixed(1)}% used)`,
      width / 2,
      barY + barH / 2,
    );

    // Remaining label
    ctx.textAlign = 'right';
    ctx.fillStyle = LABEL_COLOR;
    ctx.font = '9px monospace';
    ctx.textBaseline = 'top';
    ctx.fillText(`Remaining: ${formatPeso(budgetRemaining)}`, width - padding.right, barY + barH + 3);
    ctx.textAlign = 'start';

    // --- Breakdown by Category (stacked bar) ---
    const breakdownY = barY + barH + 22;
    const breakdownH = 16;

    // Compute totals per category
    const categoryTotals = new Map<string, number>();
    for (const evt of interventionLog) {
      const cat = interventionCategory(evt.type);
      categoryTotals.set(cat, (categoryTotals.get(cat) ?? 0) + evt.cost);
    }

    const nonEmptyCategories = CATEGORIES.filter((c) => (categoryTotals.get(c.key) ?? 0) > 0);

    if (nonEmptyCategories.length > 0 && budgetSpent > 0) {
      // Label
      ctx.font = '9px monospace';
      ctx.fillStyle = LABEL_COLOR;
      ctx.textBaseline = 'bottom';
      ctx.fillText('Spending breakdown:', padding.left, breakdownY - 2);

      // Stacked horizontal bar
      let offsetX = padding.left;

      for (const cat of nonEmptyCategories) {
        const amount = categoryTotals.get(cat.key) ?? 0;
        const fraction = amount / budgetSpent;
        const segW = Math.max(2, fraction * barW);

        ctx.fillStyle = cat.fillColor;
        ctx.fillRect(offsetX, breakdownY, segW, breakdownH);

        // Border
        ctx.strokeStyle = cat.color;
        ctx.lineWidth = 1;
        ctx.strokeRect(offsetX, breakdownY, segW, breakdownH);

        // Label inside if wide enough
        if (segW > 40) {
          ctx.font = '8px monospace';
          ctx.fillStyle = '#e5e7eb';
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillText(cat.key, offsetX + segW / 2, breakdownY + breakdownH / 2);
        }

        offsetX += segW;
      }

      // Legend below stacked bar
      let legendX = padding.left;
      const legendY = breakdownY + breakdownH + 10;
      ctx.font = '9px monospace';
      ctx.textBaseline = 'middle';
      ctx.textAlign = 'left';

      for (const cat of nonEmptyCategories) {
        const amount = categoryTotals.get(cat.key) ?? 0;

        // Color swatch
        ctx.fillStyle = cat.fillColor;
        ctx.fillRect(legendX, legendY - 3, 8, 8);
        ctx.strokeStyle = cat.color;
        ctx.lineWidth = 1;
        ctx.strokeRect(legendX, legendY - 3, 8, 8);
        legendX += 11;

        // Label + amount
        ctx.fillStyle = LABEL_COLOR;
        const text = `${cat.key}: ${formatPeso(amount)}`;
        ctx.fillText(text, legendX, legendY);
        legendX += ctx.measureText(text).width + 14;

        // Wrap to next line if too wide
        if (legendX > width - 60) {
          legendX = padding.left;
        }
      }
    } else {
      ctx.font = '9px monospace';
      ctx.fillStyle = LABEL_COLOR;
      ctx.textBaseline = 'top';
      ctx.fillText('No spending recorded yet', padding.left, breakdownY);
    }

    // --- Mini pie chart (bottom-right) ---
    if (nonEmptyCategories.length > 1 && budgetSpent > 0) {
      const pieR = 28;
      const pieCx = width - padding.right - pieR - 4;
      const pieCy = breakdownY + pieR + 10;

      let startAngle = -Math.PI / 2;
      for (const cat of nonEmptyCategories) {
        const amount = categoryTotals.get(cat.key) ?? 0;
        const fraction = amount / budgetSpent;
        const endAngle = startAngle + fraction * Math.PI * 2;

        ctx.beginPath();
        ctx.moveTo(pieCx, pieCy);
        ctx.arc(pieCx, pieCy, pieR, startAngle, endAngle);
        ctx.closePath();
        ctx.fillStyle = cat.fillColor;
        ctx.fill();
        ctx.strokeStyle = cat.color;
        ctx.lineWidth = 1;
        ctx.stroke();

        // Percentage label if slice is big enough
        if (fraction > 0.1) {
          const midAngle = (startAngle + endAngle) / 2;
          const lx = pieCx + Math.cos(midAngle) * pieR * 0.6;
          const ly = pieCy + Math.sin(midAngle) * pieR * 0.6;
          ctx.font = '8px monospace';
          ctx.fillStyle = '#e5e7eb';
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillText(`${(fraction * 100).toFixed(0)}%`, lx, ly);
        }

        startAngle = endAngle;
      }
    }

    ctx.textAlign = 'start';
  }, []);

  const budgetRemaining = useSthStore((s) => s.budgetRemaining);
  const budgetSpent = useSthStore((s) => s.budgetSpent);
  const logLength = useSthStore((s) => s.interventionLog.length);

  useEffect(() => {
    drawChart();
  }, [budgetRemaining, budgetSpent, logLength, drawChart]);

  useEffect(() => {
    drawChart();
    const interval = setInterval(drawChart, RENDER_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [drawChart]);

  useEffect(() => {
    const handleResize = () => drawChart();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [drawChart]);

  return (
    <div
      ref={containerRef}
      style={{
        width: '100%',
        height: `${CANVAS_HEIGHT}px`,
        backgroundColor: BG_COLOR,
        borderRadius: '4px',
        overflow: 'hidden',
      }}
    >
      <canvas ref={canvasRef} />
    </div>
  );
}

/** Draw a rounded rectangle path (does not fill/stroke) */
function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
): void {
  if (w < 2 * r) r = w / 2;
  if (h < 2 * r) r = h / 2;
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}
