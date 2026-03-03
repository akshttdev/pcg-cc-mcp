/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: ["class"],
  content: [
    './pages/**/*.{ts,tsx}',
    './components/**/*.{ts,tsx}',
    './app/**/*.{ts,tsx}',
    './src/**/*.{ts,tsx}',
    "node_modules/@rjsf/shadcn/src/**/*.{js,ts,jsx,tsx,mdx}"
  ],
  safelist: [
    'xl:hidden',
    'xl:relative',
    'xl:inset-auto',
    'xl:z-auto',
    'xl:h-full',
    'xl:w-[800px]',
    'xl:flex',
    'xl:flex-1',
    'xl:min-w-0',
    'xl:overflow-y-auto',
    'xl:opacity-100',
    'xl:pointer-events-auto',
    'lg:hidden',
    'lg:flex',
    'lg:relative',
    'lg:translate-x-0',
    'lg:w-64',
    'lg:block',
  ],
  prefix: "",
  theme: {
    container: {
      center: true,
      padding: "2rem",
      screens: {
        "2xl": "1400px",
      },
    },
    extend: {
      backgroundImage: {
        'diagonal-lines': `
          repeating-linear-gradient(-45deg, hsl(var(--border) / 0.4) 0 2px, transparent 1px 12px),
          linear-gradient(hsl(var(--background)), hsl(var(--background)))
        `,
      },
      ringColor: {
        DEFAULT: 'hsl(var(--primary))', // e.g. Tailwind's blue-500
      },
      fontSize: { // These are downshifted by 1
        xs: ['0.625rem', { lineHeight: '0.875rem' }], // 10px / 14px
        sm: ['0.75rem', { lineHeight: '1rem' }],     // 12px / 16px
        base: ['0.875rem', { lineHeight: '1.25rem' }],  // 14px / 20px
        lg: ['1rem', { lineHeight: '1.5rem' }],   // 16px / 24px
        xl: ['1.125rem', { lineHeight: '1.75rem' }],  // 18px / 28px
      },
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
        success: {
          DEFAULT: "hsl(var(--success))",
          foreground: "hsl(var(--success-foreground))",
        },
        warning: {
          DEFAULT: "hsl(var(--warning))",
          foreground: "hsl(var(--warning-foreground))",
        },
        info: {
          DEFAULT: "hsl(var(--info))",
          foreground: "hsl(var(--info-foreground))",
        },
        neutral: {
          DEFAULT: "hsl(var(--neutral))",
          foreground: "hsl(var(--neutral-foreground))",
        },
        status: {
          init: "hsl(var(--status-init))",
          "init-foreground": "hsl(var(--status-init-foreground))",
          running: "hsl(var(--status-running))",
          "running-foreground": "hsl(var(--status-running-foreground))",
          complete: "hsl(var(--status-complete))",
          "complete-foreground": "hsl(var(--status-complete-foreground))",
          failed: "hsl(var(--status-failed))",
          "failed-foreground": "hsl(var(--status-failed-foreground))",
          paused: "hsl(var(--status-paused))",
          "paused-foreground": "hsl(var(--status-paused-foreground))",
        },
        console: {
          DEFAULT: "hsl(var(--console-background))",
          foreground: "hsl(var(--console-foreground))",
          success: "hsl(var(--console-success))",
          error: "hsl(var(--console-error))",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      fontFamily: {
        'chivo-mono': ['Chivo Mono', 'Noto Emoji', 'monospace'],
      },
      keyframes: {
        "accordion-down": {
          from: { height: "0" },
          to: { height: "var(--radix-accordion-content-height)" },
        },
        "accordion-up": {
          from: { height: "var(--radix-accordion-content-height)" },
          to: { height: "0" },
        },
        pill: {
          '0%': { opacity: '0' },
          '10%': { opacity: '1' },
          '80%': { opacity: '1' },
          '100%': { opacity: '0' },
        },
        "fade-in": {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
        "fade-out": {
          from: { opacity: "1" },
          to: { opacity: "0" },
        },
        "slide-in-left": {
          from: { transform: "translateX(-100%)" },
          to: { transform: "translateX(0)" },
        },
        "slide-out-left": {
          from: { transform: "translateX(0)" },
          to: { transform: "translateX(-100%)" },
        },
        "slide-up": {
          from: { transform: "translateY(8px)", opacity: "0" },
          to: { transform: "translateY(0)", opacity: "1" },
        },
        "scale-in": {
          from: { transform: "scale(0.95)", opacity: "0" },
          to: { transform: "scale(1)", opacity: "1" },
        },
        shimmer: {
          '0%': { backgroundPosition: '-200% 0' },
          '100%': { backgroundPosition: '200% 0' },
        },
        pulse: {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0.5' },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
        pill: 'pill 2s ease-in-out forwards',
        "fade-in": "fade-in 0.2s ease-out",
        "fade-out": "fade-out 0.15s ease-in",
        "slide-in-left": "slide-in-left 0.25s ease-out",
        "slide-out-left": "slide-out-left 0.2s ease-in",
        "slide-up": "slide-up 0.3s ease-out",
        "scale-in": "scale-in 0.2s ease-out",
        shimmer: "shimmer 2s linear infinite",
        pulse: "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite",
      },
    },
  },
  plugins: [require("tailwindcss-animate"), require("@tailwindcss/container-queries")],
}
