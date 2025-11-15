import type { Metadata } from "next";
import { Toaster } from "sonner";
import "./globals.css";

export const metadata: Metadata = {
  title: "EVNA Blocks - AI Workspace",
  description: "Block-based AI workspace with continuous note-taking and BBS integration",
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
        <Toaster position="bottom-right" />
      </body>
    </html>
  );
}
