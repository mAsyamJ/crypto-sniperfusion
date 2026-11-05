import type { Config } from 'tailwindcss';

/** Tokens map to CSS variables defined in src/index.css (DESIGN.md §3,
 * contrast-measured pairs). No default palette leaks into the UI. */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        bg: 'var(--bg)',
        surface: 'var(--surface)',
        surface2: 'var(--surface-2)',
        border: 'var(--border)',
        fg: 'var(--fg)',
        muted: 'var(--fg-muted)',
        accent: 'var(--accent)',
        onaccent: 'var(--on-accent)',
        pos: 'var(--pos)',
        neg: 'var(--neg)',
      },
      fontFamily: {
        ui: ['"Fira Sans"', 'system-ui', 'sans-serif'],
        data: ['"Fira Code"', 'ui-monospace', 'monospace'],
      },
    },
  },
  plugins: [],
} satisfies Config;
