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
        'gradient-radial': 'radial-gradient(var(--tw-gradient-stops))',
      },
      ringColor: {
        DEFAULT: 'hsl(var(--primary))',
      },
      fontSize: {
        xs:   ['0.75rem',  { lineHeight: '1rem' }],       // 12px
        sm:   ['0.8125rem', { lineHeight: '1.25rem' }],    // 13px
        base: ['0.875rem', { lineHeight: '1.375rem' }],    // 14px
        md:   ['0.9375rem', { lineHeight: '1.5rem' }],     // 15px
        lg:   ['1rem',     { lineHeight: '1.5rem' }],      // 16px
        xl:   ['1.125rem', { lineHeight: '1.75rem' }],     // 18px
        '2xl': ['1.375rem', { lineHeight: '1.875rem' }],   // 22px
        '3xl': ['1.75rem',  { lineHeight: '2.25rem' }],    // 28px
        '4xl': ['2.25rem',  { lineHeight: '2.75rem' }],    // 36px
      },
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        brand: {
          DEFAULT: "hsl(var(--brand))",
          foreground: "hsl(var(--brand-foreground))",
        },
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
        surface: {
          0: "hsl(var(--surface-0))",
          1: "hsl(var(--surface-1))",
          2: "hsl(var(--surface-2))",
          3: "hsl(var(--surface-3))",
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
        xl: "calc(var(--radius) + 4px)",
      },
      fontFamily: {
        sans: ['"DM Sans"', 'system-ui', 'sans-serif'],
        'chivo-mono': ['Chivo Mono', 'Noto Emoji', 'monospace'],
        mono: ['Chivo Mono', 'ui-monospace', 'monospace'],
      },
      boxShadow: {
        'elevation-1': '0 1px 2px 0 rgb(0 0 0 / 0.04), 0 1px 3px 0 rgb(0 0 0 / 0.03)',
        'elevation-2': '0 2px 8px -2px rgb(0 0 0 / 0.08), 0 2px 4px -2px rgb(0 0 0 / 0.04)',
        'elevation-3': '0 8px 24px -6px rgb(0 0 0 / 0.12), 0 4px 8px -4px rgb(0 0 0 / 0.06)',
        'glow-brand': '0 0 20px -4px hsl(var(--brand) / 0.25)',
        'glow-success': '0 0 20px -4px hsl(var(--success) / 0.25)',
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
          from: { transform: "translateY(6px)", opacity: "0" },
          to: { transform: "translateY(0)", opacity: "1" },
        },
        "slide-down": {
          from: { transform: "translateY(-6px)", opacity: "0" },
          to: { transform: "translateY(0)", opacity: "1" },
        },
        "scale-in": {
          from: { transform: "scale(0.96)", opacity: "0" },
          to: { transform: "scale(1)", opacity: "1" },
        },
        shimmer: {
          '0%': { backgroundPosition: '-200% 0' },
          '100%': { backgroundPosition: '200% 0' },
        },
        "float": {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-4px)' },
        },
        "glow-pulse": {
          '0%, 100%': { boxShadow: '0 0 4px 0px hsl(var(--brand) / 0.2)' },
          '50%': { boxShadow: '0 0 16px 2px hsl(var(--brand) / 0.15)' },
        },
        "spin-slow": {
          from: { transform: "rotate(0deg)" },
          to: { transform: "rotate(360deg)" },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
        pill: 'pill 2s ease-in-out forwards',
        "fade-in": "fade-in 0.2s ease-out",
        "fade-out": "fade-out 0.15s ease-in",
        "slide-in-left": "slide-in-left 0.25s cubic-bezier(0.4, 0, 0.2, 1)",
        "slide-out-left": "slide-out-left 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
        "slide-up": "slide-up 0.3s cubic-bezier(0.4, 0, 0.2, 1)",
        "slide-down": "slide-down 0.3s cubic-bezier(0.4, 0, 0.2, 1)",
        "scale-in": "scale-in 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
        shimmer: "shimmer 2s linear infinite",
        float: "float 3s ease-in-out infinite",
        "glow-pulse": "glow-pulse 2s ease-in-out infinite",
        "spin-slow": "spin-slow 3s linear infinite",
      },
    },
  },
  plugins: [require("tailwindcss-animate"), require("@tailwindcss/container-queries")],
}
