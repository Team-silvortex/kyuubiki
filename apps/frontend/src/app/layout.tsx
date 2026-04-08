import type { Metadata } from "next";
import brand from "../../../../assets/brand/brand.json";
import "./globals.css";

const metadataBase =
  process.env.NEXT_PUBLIC_APP_URL && process.env.NEXT_PUBLIC_APP_URL.trim().length > 0
    ? new URL(process.env.NEXT_PUBLIC_APP_URL)
    : new URL("http://127.0.0.1:3000");

export const metadata: Metadata = {
  metadataBase,
  title: brand.applicationName,
  description: brand.workbenchDescription,
  applicationName: brand.productName,
  icons: {
    icon: [
      { url: "/icon.png", type: "image/png" },
      { url: "/kyuubiki.png", type: "image/png" },
    ],
    apple: [{ url: "/apple-icon.png", type: "image/png" }],
    shortcut: ["/icon.png"],
  },
  openGraph: {
    title: brand.applicationName,
    description: brand.socialDescription,
    images: [{ url: "/kyuubiki.png", type: "image/png", width: 1024, height: 1024 }],
  },
  twitter: {
    card: "summary",
    title: brand.applicationName,
    description: brand.socialDescription,
    images: ["/kyuubiki.png"],
  },
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
