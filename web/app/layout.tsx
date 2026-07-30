import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "OS Arena — Old School Magic Simulator",
  description:
    "Play deterministic Old School 93/94 Magic against Rust-powered bots.",
  openGraph: {
    title: "OS Arena",
    description: "Old cards. Exact rules. No takebacks.",
    images: ["/og.png"],
  },
  twitter: {
    card: "summary_large_image",
    title: "OS Arena",
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
