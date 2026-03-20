'use client';

import { useCallback, useEffect, useRef } from 'react';
import { useSthStore } from '@/stores/sth-store';
import { THESIS_KAP_MEANS } from '@/lib/thesis-data';

const BG_COLOR = '#1a1a2e';
const GRID_COLOR = 'rgba(255, 255, 255, 0.08)';
const LABEL_COLOR = 'rgba(255, 255, 255, 0.5)';
const THESIS_MARKER_COLOR = 'rgba(255, 255, 255, 0.6)';

const CANVAS_HEIGHT = 180;
const RENDER_INTERVAL_MS = 500;

const KAP_BARS: { key: 'meanKnowledge' | 'meanAttitude' | 'meanPractice'; label: string; color: string; fillColor: string }[] = [
  { key: 'meanKnowledge', label: 'Knowledge', color: 'rgba(59, 130, 246, 1)', fillColor: 'rgba(59, 130, 246, 0.7)' },
  { key: 'meanAttitude', label: 'Attitude', color: 'rgba(34, 197, 94, 1)', fillColor: 'rgba(34, 197, 94, 0.7)' },
  { key: 'meanPractice', label: 'Practice', color: 'rgba(249, 115, 22, 1)', fillColor: 'rgba(249, 115, 22, 0.7)' },
];

// Reference levels for KAP (horizontal lines)
const LEVELS: { label: string; value: number }[] = [
  { label: 'Poor', value: 25 },
  { label: 'Moderate', value: 50 },
  { label: 'Good', value: 75 },
  { label: 'Excellent', value: 100 },
];

export function KapChart() {
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

    const targetW = Math.round(width * dpr);
    const targetH = Math.round(CANVAS_HEIGHT * dpr);
    if (canvas.width !== targetW || canvas.height !== targetH) {
      canvas.width = targetW;
      canvas.height = targetH;
      canvas.style.width = `${width}px`;
      canvas.style.height = `${CANVAS_HEIGHT}px`;
    }

    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.resetTransform();
    ctx.scale(dpr, dpr);

    ctx.fillStyle = BG_COLOR;
    ctx.fillRect(0, 0, width, CANVAS_HEIGHT);

    const stats = useSthStore.getState().stats;

    const padding = { top: 24, bottom: 24, left: 36, right: 10 };
    const chartW = width - padding.left - padding.right;
    const chartH = CANVAS_HEIGHT - padding.top - padding.bottom;

    // Title
    ctx.font = '10px monospace';
    ctx.textBaseline = 'top';
    ctx.fillStyle = LABEL_COLOR;
    ctx.fillText('Knowledge, Attitudes & Practices', padding.left, 4);

    // Y-axis grid + labels
    ctx.strokeStyle = GRID_COLOR;
    ctx.lineWidth = 1;
    ctx.font = '9px monospace';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';

    for (const level of LEVELS) {
      const y = padding.top + chartH * (1 - level.value / 100);
      ctx.fillStyle = LABEL_COLOR;
      ctx.fillText(`${level.value}`, padding.left - 4, y);

      // Dashed reference line with label
      ctx.beginPath();
      ctx.setLineDash([3, 4]);
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
      ctx.moveTo(padding.left, y);
      ctx.lineTo(padding.left + chartW, y);
      ctx.stroke();
      ctx.setLineDash([]);

      // Level label on right
      ctx.textAlign = 'left';
      ctx.fillStyle = 'rgba(255, 255, 255, 0.25)';
      ctx.font = '8px monospace';
      ctx.fillText(level.label, padding.left + chartW - ctx.measureText(level.label).width - 2, y);
      ctx.font = '9px monospace';
      ctx.textAlign = 'right';
    }

    // Zero line
    ctx.fillStyle = LABEL_COLOR;
    ctx.textAlign = 'right';
    ctx.fillText('0', padding.left - 4, padding.top + chartH);

    if (!stats) {
      ctx.font = '11px monospace';
      ctx.fillStyle = LABEL_COLOR;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('Waiting for data...', width / 2, CANVAS_HEIGHT / 2);
      ctx.textAlign = 'start';
      return;
    }

    // Draw bars
    const barGroupWidth = chartW / KAP_BARS.length;
    const barWidth = Math.min(60, barGroupWidth * 0.6);
    const barGap = (barGroupWidth - barWidth) / 2;

    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';

    for (let i = 0; i < KAP_BARS.length; i++) {
      const bar = KAP_BARS[i];
      const value = stats[bar.key] ?? 0;
      const clampedValue = Math.max(0, Math.min(100, value));
      const barHeight = (clampedValue / 100) * chartH;
      const x = padding.left + i * barGroupWidth + barGap;
      const y = padding.top + chartH - barHeight;

      // Bar fill
      ctx.fillStyle = bar.fillColor;
      ctx.fillRect(x, y, barWidth, barHeight);

      // Bar border
      ctx.strokeStyle = bar.color;
      ctx.lineWidth = 1;
      ctx.strokeRect(x, y, barWidth, barHeight);

      // Value on top of bar
      ctx.font = '10px monospace';
      ctx.fillStyle = bar.color;
      ctx.textBaseline = 'bottom';
      ctx.fillText(`${clampedValue.toFixed(1)}%`, x + barWidth / 2, y - 2);

      // Label below bar
      ctx.font = '9px monospace';
      ctx.fillStyle = LABEL_COLOR;
      ctx.textBaseline = 'top';
      ctx.fillText(bar.label, x + barWidth / 2, padding.top + chartH + 4);

      // Thesis reference marker — small triangle + dashed line across the bar
      const thesisMap: Record<string, number> = {
        meanKnowledge: THESIS_KAP_MEANS.knowledge,
        meanAttitude: THESIS_KAP_MEANS.attitude,
        meanPractice: THESIS_KAP_MEANS.practice,
      };
      const thesisVal = thesisMap[bar.key];
      if (thesisVal !== undefined && thesisVal > 0) {
        const thesisY = padding.top + chartH * (1 - thesisVal / 100);

        // Dashed line across bar width
        ctx.save();
        ctx.setLineDash([3, 2]);
        ctx.strokeStyle = THESIS_MARKER_COLOR;
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(x - 4, thesisY);
        ctx.lineTo(x + barWidth + 4, thesisY);
        ctx.stroke();
        ctx.setLineDash([]);

        // Small triangle marker on left edge
        ctx.fillStyle = THESIS_MARKER_COLOR;
        ctx.beginPath();
        ctx.moveTo(x - 6, thesisY);
        ctx.lineTo(x - 1, thesisY - 3);
        ctx.lineTo(x - 1, thesisY + 3);
        ctx.closePath();
        ctx.fill();

        // Label "T" next to marker
        ctx.font = '7px monospace';
        ctx.textAlign = 'right';
        ctx.textBaseline = 'middle';
        ctx.fillStyle = THESIS_MARKER_COLOR;
        ctx.fillText('T', x - 8, thesisY);
        ctx.restore();
      }
    }

    ctx.textAlign = 'start';
  }, []);

  // Re-draw on stats change
  const meanKnowledge = useSthStore((s) => s.stats?.meanKnowledge);
  const meanAttitude = useSthStore((s) => s.stats?.meanAttitude);
  const meanPractice = useSthStore((s) => s.stats?.meanPractice);

  useEffect(() => {
    drawChart();
  }, [meanKnowledge, meanAttitude, meanPractice, drawChart]);

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
