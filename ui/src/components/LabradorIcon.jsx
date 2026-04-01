/** Front-facing labrador retriever icon. Pass `color` and `size`. */
export default function LabradorIcon({ color = '#c4922a', size = 48, status = 'idle' }) {
  const ear = color;
  const head = color;
  const snout = lighten(color, 0.22);
  const glow = status === 'running' ? color : 'transparent';

  return (
    <svg
      viewBox="0 0 48 48"
      width={size}
      height={size}
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={{ overflow: 'visible', filter: status === 'running' ? `drop-shadow(0 0 6px ${color}88)` : undefined }}
    >
      {/* Glow ring when running */}
      {status === 'running' && (
        <circle cx="24" cy="22" r="22" stroke={glow} strokeWidth="1.5" strokeOpacity="0.4" fill="none" />
      )}

      {/* Left ear */}
      <ellipse cx="8.5" cy="26" rx="6.5" ry="11" fill={ear} opacity="0.78"
        transform="rotate(-8 8.5 26)" />

      {/* Right ear */}
      <ellipse cx="39.5" cy="26" rx="6.5" ry="11" fill={ear} opacity="0.78"
        transform="rotate(8 39.5 26)" />

      {/* Head */}
      <ellipse cx="24" cy="20" rx="17" ry="15.5" fill={head} />

      {/* Snout area */}
      <ellipse cx="24" cy="29" rx="10" ry="7.5" fill={snout} />

      {/* Left eye white */}
      <circle cx="17.5" cy="17.5" r="4.5" fill="rgba(255,255,255,0.18)" />
      {/* Right eye white */}
      <circle cx="30.5" cy="17.5" r="4.5" fill="rgba(255,255,255,0.18)" />

      {/* Left eye */}
      <circle cx="17.5" cy="17.5" r="3.2" fill="rgba(0,0,0,0.72)" />
      {/* Right eye */}
      <circle cx="30.5" cy="17.5" r="3.2" fill="rgba(0,0,0,0.72)" />

      {/* Eye shine left */}
      <circle cx="18.8" cy="16.3" r="1.1" fill="rgba(255,255,255,0.75)" />
      {/* Eye shine right */}
      <circle cx="31.8" cy="16.3" r="1.1" fill="rgba(255,255,255,0.75)" />

      {/* Nose */}
      <ellipse cx="24" cy="26.5" rx="5.5" ry="3.8" fill="rgba(0,0,0,0.62)" />
      {/* Nose shine */}
      <ellipse cx="22.5" cy="25.2" rx="2" ry="1.2" fill="rgba(255,255,255,0.28)" />

      {/* Mouth */}
      <path d="M 19.5 31.5 Q 24 36 28.5 31.5" stroke="rgba(0,0,0,0.38)" strokeWidth="1.6" fill="none" strokeLinecap="round" />

      {/* Forehead crease (friendly) */}
      <path d="M 20 12 Q 24 10 28 12" stroke="rgba(0,0,0,0.15)" strokeWidth="1.2" fill="none" strokeLinecap="round" />
    </svg>
  );
}

/** Naive color lightening for snout — just shifts toward white */
function lighten(hex, amount) {
  const num = parseInt(hex.replace('#', ''), 16);
  const r = Math.min(255, ((num >> 16) & 0xff) + Math.round(255 * amount));
  const g = Math.min(255, ((num >> 8) & 0xff) + Math.round(255 * amount));
  const b = Math.min(255, (num & 0xff) + Math.round(255 * amount));
  return `rgb(${r},${g},${b})`;
}
