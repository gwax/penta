import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Penta — Old School Magic Simulator",
  description:
    "Play deterministic Old School 93/94 Magic against Rust-powered bots.",
  openGraph: {
    title: "Penta",
    description: "Old cards. Exact rules. No takebacks.",
    images: ["/og.png"],
  },
  twitter: {
    card: "summary_large_image",
    title: "Penta",
    description: "Old cards. Exact rules. No takebacks.",
    images: ["/og.png"],
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
