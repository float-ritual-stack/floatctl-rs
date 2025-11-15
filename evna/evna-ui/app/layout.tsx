import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "EVNA Block Chat",
  description: "Block-based AI agent interface for EVNA",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased">
        {children}
      </body>
    </html>
  );
}
