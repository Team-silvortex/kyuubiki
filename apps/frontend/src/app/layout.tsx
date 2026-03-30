import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Kyuubiki Workbench",
  description: "Kyuubiki browser workbench for orchestrated FEM studies",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
