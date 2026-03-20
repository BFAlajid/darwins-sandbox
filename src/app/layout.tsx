import type { Metadata } from 'next';
import './globals.css';
import { ErrorBoundary } from '@/components/ui/error-boundary';

export const metadata: Metadata = {
  title: 'STH Simulation — Soil-Transmitted Helminth Infection & Deworming',
  description: 'Agent-based simulation of STH infection dynamics, WASH interventions, and deworming programs in Philippine barangays. Built with Rust/WASM.',
  metadataBase: new URL('https://darwins-sandbox.vercel.app'),
  openGraph: {
    title: 'STH Simulation — Infection & Deworming',
    description: 'Interactive simulation of soil-transmitted helminth transmission, KAP behavioral model, and intervention strategies. Rust/WASM agent-based model.',
    type: 'website',
    siteName: 'STH Simulation',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'STH Simulation — Infection & Deworming',
    description: 'Agent-based STH simulation with MDA, WASH, and education interventions. Rust/WASM.',
  },
  keywords: ['STH', 'soil-transmitted helminth', 'deworming', 'WASH', 'simulation', 'agent-based model', 'WASM', 'Rust', 'public health', 'Cebu'],
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="bg-[#0a0a1a] text-gray-200 antialiased">
        <ErrorBoundary>
          {children}
        </ErrorBoundary>
      </body>
    </html>
  );
}
