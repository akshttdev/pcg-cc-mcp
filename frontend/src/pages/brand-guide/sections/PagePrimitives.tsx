/** Primitives shared across multiple brand guide pages */

export function PageBreak() {
  return <div className="page-break" />;
}

export function PageLabel({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-[9px] uppercase tracking-[0.25em] text-gray-300 mb-8 font-medium">{children}</p>
  );
}
