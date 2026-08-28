/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        accent: "#0078D4",
        "accent-hover": "#005A9E",
        "bg-mica": "rgba(255, 255, 255, 0.8)",
        "bg-mica-dark": "rgba(30, 30, 30, 0.8)",
      },
      backdropBlur: {
        mica: "20px",
      }
    },
  },
  plugins: [],
}
