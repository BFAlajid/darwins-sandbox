'use client';

import { useCallback, useEffect, useRef } from 'react';
import { useSthStore } from '@/stores/sth-store';
import { THESIS_PREVALENCE_LINES } from '@/lib/thesis-data';

const BG_COLOR = '#1a1a2e';
const GRID_COLOR = 'rgba(255, 255, 255, 0.08)';
const LABEL_COLOR = 'rgba(255, 255, 255, 0.5)';
const THESIS_LINE_COLOR = 'rgba(255, 255, 255, 0.45)';
const THESIS_LABEL_COLOR = 'rgba(255, 255, 255, 0.55)';

const CANVAS_HEIGHT = 180;
const RENDER_INTERVAL_MS = 500;

type PrevalenceKey = 'ascaris' | 'trichuris' | 'hookworm' | 'any';

interface LineConfig {
  key: PrevalenceKey;
  label: string;
  color: string;
  fillColor: string;
}

const LINES: LineConfig[] = [
  { key: 'any', label: 'Any STH', color: 'rgba(220, 220, 220, 1)', fillColor: 'rgba(220, 220, 220, 0.7)' },
  { key: 'ascaris', label: 'Ascaris', color: 'rgba(239, 68, 68, 1)', fillColor: 'rgba(239, 68, 68, 0.7)' },
  { key: 'trichuris', label: 'Trichuris', color: 'rgba(59, 130, 246, 1)', fillColor: 'rgba(59, 130, 246, 0.7)' },
  { key: 'hookworm', label: 'Hookworm', color: 'rgba(34, 197, 94, 1)', fillColor: 'rgba(34, 197, 94, 0.7)' },
];

export function PrevalenceChart() {
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

    const history = useSthStore.getState().prevalenceHistory;
    if (history.length === 0) {
      ctx.font = '11px monospace';
      ctx.fillStyle = LABEL_COLOR;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('Waiting for data...', width / 2, CANVAS_HEIGHT / 2);
      ctx.textAlign = 'start';
      return;
    }

    const padding = { top: 24, bottom: 20, left: 36, right: 10 };
    const chartW = width - padding.left - padding.right;
    const chartH = CANVAS_HEIGHT - padding.top - padding.bottom;

    // Title
    ctx.font = '10px monospace';
    ctx.textBaseline = 'top';
    ctx.fillStyle = LABEL_COLOR;
    ctx.fillText('STH Prevalence (%)', padding.left, 4);

    // Y-axis: 0-100%
    ctx.strokeStyle = GRID_COLOR;
    ctx.lineWidth = 1;
    ctx.font = '9px monospace';
    ctx.fillStyle = LABEL_COLOR;
    ctx.textBaseline = 'middle';
    ctx.textAlign = 'right';

    for (const pct of [0, 25, 50, 75, 100]) {
      const y = padding.top + chartH * (1 - pct / 100);
      ctx.beginPath();
      ctx.moveTo(padding.left, y);
      ctx.lineTo(padding.left + chartW, y);
      ctx.stroke();
      ctx.fillText(`${pct}`, padding.left - 4, y);
    }

    // X-axis labels
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    const lastDay = history[history.length - 1].day;
    const xStep = chartW / (history.length > 1 ? history.length - 1 : 1);

    // Show ~5 x-axis labels
    const labelInterval = Math.max(1, Math.floor(history.length / 5));
    for (let i = 0; i < history.length; i += labelInterval) {
      const x = padding.left + i * xStep;
      ctx.fillText(`D${history[i].day}`, x, padding.top + chartH + 4);
    }
    // Always label the last point
    if (history.length > 1) {
      const x = padding.left + (history.length - 1) * xStep;
      ctx.fillText(`D${lastDay}`, x, padding.top + chartH + 4);
    }

    // Draw lines
    ctx.textAlign = 'start';
    for (const line of LINES) {
      ctx.beginPath();
      ctx.strokeStyle = line.color;
      ctx.lineWidth = line.key === 'any' ? 2 : 1.5;

      if (line.key === 'any') {
        ctx.setLineDash([4, 3]);
      } else {
        ctx.setLineDash([]);
      }

      for (let i = 0; i < history.length; i++) {
        const x = padding.left + i * xStep;
        const val = history[i][line.key];
        const y = padding.top + chartH * (1 - val / 100);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Thesis reference lines (dashed, semi-transparent)
    ctx.save();
    ctx.setLineDash([6, 4]);
    ctx.lineWidth = 1;
    ctx.font = '8px monospace';
    ctx.textBaseline = 'middle';

    for (const ref of THESIS_PREVALENCE_LINES) {
      if (ref.value <= 0) continue; // Skip hookworm (0%)
      const y = padding.top + chartH * (1 - ref.value / 100);

      // Draw dashed line
      ctx.strokeStyle = THESIS_LINE_COLOR;
      ctx.beginPath();
      ctx.moveTo(padding.left, y);
      ctx.lineTo(padding.left + chartW, y);
      ctx.stroke();

      // Label on the left edge
      ctx.textAlign = 'left';
      ctx.fillStyle = THESIS_LABEL_COLOR;
      ctx.fillText(`${ref.value}%`, padding.left + 2, y - 7);
    }

    ctx.setLineDash([]);
    ctx.restore();

    // Legend (top-right)
    const legendX = width - padding.right - 4;
    let legendY = padding.top + 2;
    ctx.font = '9px monospace';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'top';

    for (const line of LINES) {
      const lastVal = history[history.length - 1][line.key];
      ctx.fillStyle = line.fillColor;
      ctx.fillRect(legendX - ctx.measureText(`${line.label} ${lastVal.toFixed(1)}%`).width - 10, legendY + 2, 6, 6);
      ctx.fillStyle = line.fillColor;
      ctx.fillText(`${line.label} ${lastVal.toFixed(1)}%`, legendX, legendY);
      legendY += 13;
    }

    ctx.textAlign = 'start';
  }, []);

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

  // Re-draw when prevalence history updates
  const historyLen = useSthStore((s) => s.prevalenceHistory.length);
  useEffect(() => {
    drawChart();
  }, [historyLen, drawChart]);

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
