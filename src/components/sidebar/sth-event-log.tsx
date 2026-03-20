'use client';

import { useEffect, useRef } from 'react';
import { useSthStore } from '@/stores/sth-store';
import type { SthEvent } from '@/lib/sth-types';

/** Map event types to display color classes */
function eventColor(eventType: string): string {
  switch (eventType) {
    case 'mda_completed':
    case 'mda_launched':
      return 'text-blue-400';
    case 'education_session':
      return 'text-purple-400';
    case 'latrine_built':
    case 'water_pump_built':
    case 'handwash_station_built':
      return 'text-amber-400';
    case 'bhw_visit':
      return 'text-teal-400';
    case 'outbreak':
      return 'text-red-400';
    case 'rain':
    case 'flood':
      return 'text-cyan-400';
    case 'budget_increase':
      return 'text-green-400';
    default:
      return 'text-gray-400';
  }
}

/** Map event types to a short icon/symbol */
function eventIcon(eventType: string): string {
  switch (eventType) {
    case 'mda_completed':
    case 'mda_launched':
      return '+';
    case 'education_session':
      return 'E';
    case 'latrine_built':
    case 'water_pump_built':
    case 'handwash_station_built':
      return 'B';
    case 'bhw_visit':
      return 'V';
    case 'outbreak':
      return '!';
    case 'rain':
    case 'flood':
      return '~';
    case 'budget_increase':
      return '$';
    default:
      return '-';
  }
}

function formatTick(tick: number): string {
  if (tick >= 10000) return `${(tick / 1000).toFixed(0)}k`;
  return tick.toLocaleString();
}

export function SthEventLog() {
  const events = useSthStore((s) => s.events);
  const currentDay = useSthStore((s) => s.currentDay);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to latest event
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [events.length]);

  if (events.length === 0) {
    return (
      <div className="p-3 text-gray-500 text-xs italic">
        Waiting for simulation events...
      </div>
    );
  }

  return (
    <div
      ref={scrollRef}
      className="flex flex-col gap-0.5 p-2 overflow-y-auto max-h-full"
    >
      {events.map((evt: SthEvent, i: number) => {
        const color = eventColor(evt.eventType);
        const icon = eventIcon(evt.eventType);

        return (
          <div
            key={i}
            className="flex items-start gap-1.5 text-xs leading-snug"
          >
            <span className="text-gray-600 font-mono flex-shrink-0 w-10 text-right">
              D{Math.floor(evt.tick / 24)}
            </span>
            <span
              className={`${color} font-mono flex-shrink-0 w-3 text-center`}
            >
              {icon}
            </span>
            <span className="text-gray-300">{evt.message}</span>
          </div>
        );
      })}
    </div>
  );
}
