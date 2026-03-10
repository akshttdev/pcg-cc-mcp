export function Logo() {
  return (
    <div className="flex items-center gap-2.5">
      <img
        src="/pcg-icon.png"
        alt="Powerclub Global"
        className="h-9 w-auto"
      />
      <div className="flex flex-col leading-none">
        <span style={{ fontFamily: "'Cinzel', serif", fontWeight: 600, letterSpacing: '0.08em', fontSize: '0.85rem', color: '#b8962e' }}>
          POWERCLUB GLOBAL
        </span>
        <span style={{ fontFamily: "'Cinzel', serif", fontWeight: 400, letterSpacing: '0.18em', fontSize: '0.6rem', color: '#888' }}>
          DASHBOARD
        </span>
      </div>
    </div>
  );
}
