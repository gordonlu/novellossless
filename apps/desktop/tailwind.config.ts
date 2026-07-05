import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "Microsoft YaHei",
          "Segoe UI",
          "PingFang SC",
          "Hiragino Sans GB",
          "Arial",
          "sans-serif",
        ],
      },
      colors: {
        paper: "#f7f5ef",
        canvas: "#fbfaf6",
        ink: "#24231f",
        graphite: "#625f58",
        line: "#ddd7c9",
        teal: "#1f7a68",
        moss: "#5d7b4d",
        amber: "#b26b22",
        danger: "#9c3f30",
      },
      boxShadow: {
        low: "0 1px 2px rgba(47, 43, 35, 0.05)",
        lift: "0 14px 34px rgba(47, 43, 35, 0.09)",
      },
    },
  },
  plugins: [],
} satisfies Config;
