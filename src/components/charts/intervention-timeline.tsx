'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { useSthStore } from '@/stores/sth-store';
import type { InterventionEvent } from '@/lib/sth-types';

const BG_COLOR = '#1a1a2e';
const GRID_COLOR = 'rgba(255, 255, 255, 0.08)';
const LABEL_COLOR = 'rgba(255, 255, 255, 0.5)';

const CANVAS_HEIGHT = 160;
const RENDER_INTERVAL_MS = 500;

// Intervention type -> color mapping
function interventionColor(type: string): string {
  const t = type.toLowerCase();
  if (t.includes('mda') || t.includes('deworm') || t.includes('drug')) return 'rgba(239, 68, 68, 0.9)';
  if (t.includes('wash') || t.includes('latrine') || t.includes('water') || t.includes('handwash')) return 'rgba(59, 130, 246, 0.9)';
  if (t.includes('edu') || t.includes('teach') || t.includes('cartoon') || t.includes('board') || t.includes('game')) return 'rgba(34, 197, 94, 0.9)';
  if (t.includes('bhw') || t.includes('health') || t.includes('visit')) return 'rgba(168, 85, 247, 0.9)';
  return 'rgba(220, 220, 220, 0.9)';
}

function interventionFillColor(type: string): string {
  const t = type.toLowerCase();
  if (t.includes('mda') || t.includes('deworm') || t.includes('drug')) return 'rgba(239, 68, 68, 0.3)';
  if (t.includes('wash') || t.includes('latrine') || t.includes('water') || t.includes('handwash')) return 'rgba(59, 130, 246, 0.3)';
  if (t.includes('edu') || t.includes('teach') || t.includes('cartoon') || t.includes('board') || t.includes('game')) return 'rgba(34, 197, 94, 0.3)';
  if (t.includes('bhw') || t.includes('health') || t.includes('visit')) return 'rgba(168, 85, 247, 0.3)';
  return 'rgba(220, 220, 220, 0.3)';
}

function interventionCategory(type: string): string {
  const t = type.toLowerCase();
  if (t.includes('mda') || t.includes('deworm') || t.includes('drug')) return 'MDA';
  if (t.includes('wash') || t.includes('latrine') || t.includes('water') || t.includes('handwash')) return 'WASH';
  if (t.includes('edu') || t.includes('teach') || t.includes('cartoon') || t.includes('board') || t.includes('game')) return 'Education';
  if (t.includes('bhw') || t.includes('health') || t.includes('visit')) return 'BHW';
  return 'Other';
}

interface HoveredEvent {
  event: InterventionEvent;
  x: number;
  y: number;
}

export function InterventionTimeline() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [hovered, setHovered] = useState<HoveredEvent | null>(null);
  const eventPositionsRef = useRef<{ event: InterventionEvent; x: number; y: number; radius: number }[]>([]);

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

    const padding = { top: 24, bottom: 24, left: 36, right: 10 };
    const chartW = width - padding.left - padding.right;
    const chartH = CANVAS_HEIGHT - padding.top - padding.bottom;

    // Title
    ctx.font = '10px monospace';
    ctx.textBaseline = 'top';
    ctx.fillStyle = LABEL_COLOR;
    ctx.fillText('Intervention Timeline', padding.left, 4);

    const events = useSthStore.getState().interventionLog;
    const currentDay = useSthStore.getState().currentDay;

    if (events.length === 0) {
      ctx.font = '11px monospace';
      ctx.fillStyle = LABEL_COLOR;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('No interventions yet', width / 2, CANVAS_HEIGHT / 2);
      ctx.textAlign = 'start';
      eventPositionsRef.current = [];
      return;
    }

    // X-axis: day 0 to currentDay (or max event day)
    const maxDay = Math.max(currentDay, ...events.map((e) => e.day));
    const minDay = 0;
    const dayRange = maxDay - minDay || 1;

    // Horizontal timeline line
    const timelineY = padding.top + chartH / 2;
    ctx.strokeStyle = GRID_COLOR;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(padding.left, timelineY);
    ctx.lineTo(padding.left + chartW, timelineY);
    ctx.stroke();

    // X-axis labels
    ctx.font = '9px monospace';
    ctx.fillStyle = LABEL_COLOR;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';

    const numLabels = Math.min(6, maxDay);
    const labelStep = Math.max(1, Math.floor(dayRange / numLabels));
    for (let d = minDay; d <= maxDay; d += labelStep) {
      const x = padding.left + ((d - minDay) / dayRange) * chartW;
      ctx.fillText(`D${d}`, x, padding.top + chartH + 4);
    }

    // Draw event markers
    const positions: { event: InterventionEvent; x: number; y: number; radius: number }[] = [];

    // Group events by category for vertical staggering
    const categoryOrder = ['MDA', 'WASH', 'Education', 'BHW', 'Other'];
    const categoryYOffsets: Record<string, number> = {};
    const bandHeight = chartH / (categoryOrder.length + 1);
    categoryOrder.forEach((cat, i) => {
      categoryYOffsets[cat] = padding.top + bandHeight * (i + 0.5);
    });

    // Category labels on right side
    ctx.font = '8px monospace';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';

    for (const cat of categoryOrder) {
      const y = categoryYOffsets[cat];
      // Faint horizontal band line
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.04)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(padding.left, y);
      ctx.lineTo(padding.left + chartW, y);
      ctx.stroke();
    }

    for (const evt of events) {
      const cat = interventionCategory(evt.type);
      const x = padding.left + ((evt.day - minDay) / dayRange) * chartW;
      const y = categoryYOffsets[cat] ?? timelineY;
      const radius = Math.min(8, Math.max(4, Math.sqrt(evt.cost / 500) * 4));

      // Vertical line from timeline to marker
      ctx.strokeStyle = interventionFillColor(evt.type);
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(x, timelineY);
      ctx.lineTo(x, y);
      ctx.stroke();

      // Circle marker
      ctx.beginPath();
      ctx.arc(x, y, radius, 0, Math.PI * 2);
      ctx.fillStyle = interventionFillColor(evt.type);
      ctx.fill();
      ctx.strokeStyle = interventionColor(evt.type);
      ctx.lineWidth = 1.5;
      ctx.stroke();

      positions.push({ event: evt, x, y, radius });
    }

    eventPositionsRef.current = positions;

    // Legend at bottom-right
    ctx.font = '8px monospace';
    ctx.textBaseline = 'middle';
    ctx.textAlign = 'right';
    let lx = width - padding.right - 4;
    const ly = CANVAS_HEIGHT - 8;

    const legendItems = [
      { label: 'MDA', color: 'rgba(239, 68, 68, 0.9)' },
      { label: 'WASH', color: 'rgba(59, 130, 246, 0.9)' },
      { label: 'Edu', color: 'rgba(34, 197, 94, 0.9)' },
      { label: 'BHW', color: 'rgba(168, 85, 247, 0.9)' },
    ];

    for (let i = legendItems.length - 1; i >= 0; i--) {
      const item = legendItems[i];
      ctx.fillStyle = LABEL_COLOR;
      ctx.fillText(item.label, lx, ly);
      lx -= ctx.measureText(item.label).width + 2;
      ctx.fillStyle = item.color;
      ctx.fillRect(lx - 5, ly - 3, 6, 6);
      lx -= 12;
    }

    ctx.textAlign = 'start';
  }, []);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;

    for (const pos of eventPositionsRef.current) {
      const dx = mx - pos.x;
      const dy = my - pos.y;
      if (dx * dx + dy * dy <= (pos.radius + 4) * (pos.radius + 4)) {
        setHovered({ event: pos.event, x: pos.x, y: pos.y });
        return;
      }
    }
    setHovered(null);
  }, []);

  const handleMouseLeave = useCallback(() => {
    setHovered(null);
  }, []);

  const logLength = useSthStore((s) => s.interventionLog.length);
  const currentDay = useSthStore((s) => s.currentDay);

  useEffect(() => {
    drawChart();
  }, [logLength, currentDay, drawChart]);

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
        position: 'relative',
      }}
    >
      <canvas
        ref={canvasRef}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
        style={{ cursor: hovered ? 'pointer' : 'default' }}
      />
      {hovered && (
        <div
          className="absolute bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-gray-200 pointer-events-none z-10 whitespace-nowrap"
          style={{
            left: Math.min(hovered.x, (containerRef.current?.getBoundingClientRect().width ?? 200) - 160),
            top: Math.max(0, hovered.y - 40),
          }}
        >
          <div className="font-bold" style={{ color: interventionColor(hovered.event.type) }}>
            {hovered.event.type}
          </div>
          <div>Day {hovered.event.day}</div>
          <div>{hovered.event.description}</div>
          <div className="text-yellow-400">
            Cost: {'\u20B1'}{hovered.event.cost.toLocaleString()}
          </div>
          {hovered.event.result && <div className="text-green-400">{hovered.event.result}</div>}
        </div>
      )}
    </div>
  );
}
