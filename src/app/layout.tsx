import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: "Darwin's Sandbox",
  description: 'Real-time evolution simulator — watch neural network creatures evolve',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="bg-[#0a0a1a] text-gray-200 antialiased">
        {children}
      </body>
    </html>
  );
}
