import type { Metadata } from "next";
import { Orbitron, Share_Tech_Mono } from "next/font/google";
import "./globals.css";

const display = Orbitron({
  subsets: ["latin"],
  variable: "--font-display",
});

const mono = Share_Tech_Mono({
  weight: "400",
  subsets: ["latin"],
  variable: "--font-mono",
});

export const metadata: Metadata = {
  title: "J.A.R.V.I.S. HUD",
  description: "Stark holographic interface",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={`${display.variable} ${mono.variable}`}>{children}</body>
    </html>
  );
}
