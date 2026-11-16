import { ICON_PATHS } from "./iconPaths";

export type IconName = keyof typeof ICON_PATHS;

/** An icon of the design set; decorative (aria-hidden): the control around it carries the label (UI-7). */
export function Icon({ name, size = "", className = "" }: { name: IconName; size?: "" | "m" | "sm"; className?: string }) {
  return (
    <svg
      className={`ic ${size} ${className}`}
      viewBox="0 0 16 16"
      aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: ICON_PATHS[name] }}
    />
  );
}

/**
 * The Stemwise mark (A-076, design/brand/final): two rooms, the Mix (two lines that come together) and, past a gap,
 * the Master (one solid room). The room of the current phase is lit in its accent, the other is dimmed (components.css).
 * `large` is the 100-grid drawing, for 24 px and up; `brand` keeps lime and lilac.
 */
export function Mark({ large = false, brand = false }: { large?: boolean; brand?: boolean }) {
  return large ? (
    <svg className={`mark ${brand ? "brand" : ""}`} viewBox="0 0 100 100" aria-hidden="true">
      <path
        className="mark-mix"
        strokeWidth="9"
        d="M9 20H23Q31 20 35.2 27.65L40.8 37.85Q45 45.5 53 45.5M9 80H23Q31 80 35.2 72.35L40.8 62.15Q45 54.5 53 54.5"
      />
      <path className="mark-master" d="M75 37H85A10 10 0 0 1 95 47V53A10 10 0 0 1 85 63H75A10 10 0 0 1 65 53V47A10 10 0 0 1 75 37Z" />
    </svg>
  ) : (
    <svg className={`mark ${brand ? "brand" : ""}`} viewBox="0 0 20 20" aria-hidden="true">
      <path
        className="mark-mix"
        strokeWidth="2"
        d="M1.75 4H4.4Q6 4 6.84 5.5L7.96 7.5Q8.8 9 10.5 9M1.75 16H4.4Q6 16 6.84 14.5L7.96 12.5Q8.8 11 10.5 11"
      />
      <path className="mark-master" d="M15 7H17.25A2 2 0 0 1 19.25 9V11A2 2 0 0 1 17.25 13H15A2 2 0 0 1 13 11V9A2 2 0 0 1 15 7Z" />
    </svg>
  );
}
