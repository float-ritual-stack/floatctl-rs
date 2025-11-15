import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "EVNA - Context Synthesis & Semantic Search",
  description: "AI agent for context synthesis and semantic search across conversation history",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased font-sans">
        {children}
      </body>
    </html>
  );
}
