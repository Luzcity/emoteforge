/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Win11 風の落ち着いた配色
        surface: "#202020",
        panel: "#2b2b2b",
        accent: "#4cc2ff",
      },
    },
  },
  plugins: [],
};
