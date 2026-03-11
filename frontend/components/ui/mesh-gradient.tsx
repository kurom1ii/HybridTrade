import { cn } from "@/lib/utils";

/* ── Preset mesh gradient themes ── */
const presets = {
  ocean: {
    bg: "rgba(0,20,40,0.9)",
    blobs: [
      { color: "rgba(34,211,238,0.35)", x: "20%", y: "30%", size: "60%" },
      { color: "rgba(99,102,241,0.25)", x: "70%", y: "20%", size: "50%" },
      { color: "rgba(139,92,246,0.2)", x: "50%", y: "80%", size: "55%" },
      { color: "rgba(6,182,212,0.15)", x: "80%", y: "60%", size: "45%" },
    ],
  },
  sunset: {
    bg: "rgba(30,10,20,0.9)",
    blobs: [
      { color: "rgba(236,72,153,0.3)", x: "25%", y: "25%", size: "55%" },
      { color: "rgba(245,158,11,0.25)", x: "75%", y: "30%", size: "50%" },
      { color: "rgba(139,92,246,0.2)", x: "40%", y: "75%", size: "60%" },
      { color: "rgba(244,63,94,0.15)", x: "65%", y: "65%", size: "45%" },
    ],
  },
  aurora: {
    bg: "rgba(5,15,25,0.9)",
    blobs: [
      { color: "rgba(34,211,238,0.3)", x: "30%", y: "20%", size: "65%" },
      { color: "rgba(34,197,94,0.2)", x: "70%", y: "40%", size: "50%" },
      { color: "rgba(99,102,241,0.25)", x: "20%", y: "70%", size: "55%" },
      { color: "rgba(6,182,212,0.15)", x: "80%", y: "80%", size: "40%" },
    ],
  },
  cyber: {
    bg: "rgba(8,8,12,0.95)",
    blobs: [
      { color: "rgba(34,211,238,0.4)", x: "15%", y: "30%", size: "50%" },
      { color: "rgba(99,102,241,0.3)", x: "85%", y: "25%", size: "45%" },
      { color: "rgba(34,211,238,0.15)", x: "50%", y: "80%", size: "60%" },
      { color: "rgba(139,92,246,0.1)", x: "60%", y: "50%", size: "35%" },
    ],
  },
  ember: {
    bg: "rgba(20,8,5,0.9)",
    blobs: [
      { color: "rgba(255,68,68,0.3)", x: "20%", y: "20%", size: "55%" },
      { color: "rgba(245,158,11,0.25)", x: "80%", y: "30%", size: "50%" },
      { color: "rgba(236,72,153,0.2)", x: "50%", y: "75%", size: "60%" },
      { color: "rgba(255,136,0,0.15)", x: "30%", y: "60%", size: "40%" },
    ],
  },
} as const;

export type MeshPreset = keyof typeof presets;

interface MeshGradientProps {
  preset?: MeshPreset;
  className?: string;
  children?: React.ReactNode;
  animate?: boolean;
  blur?: number;
  as?: "div" | "section";
}

export function MeshGradient({
  preset = "ocean",
  className,
  children,
  animate = false,
  blur = 80,
  as: Tag = "div",
}: MeshGradientProps) {
  const { bg, blobs } = presets[preset];

  return (
    <Tag className={cn("relative overflow-hidden", className)}>
      {/* Base background */}
      <div className="absolute inset-0" style={{ background: bg }} />

      {/* Gradient blobs */}
      {blobs.map((blob, i) => (
        <div
          key={i}
          className={cn("absolute rounded-full", animate && "animate-mesh-float")}
          style={{
            background: `radial-gradient(circle, ${blob.color} 0%, transparent 70%)`,
            left: blob.x,
            top: blob.y,
            width: blob.size,
            height: blob.size,
            transform: "translate(-50%, -50%)",
            filter: `blur(${blur}px)`,
            animationDelay: animate ? `${i * 1.5}s` : undefined,
            animationDuration: animate ? `${8 + i * 2}s` : undefined,
          }}
        />
      ))}

      {/* Content layer */}
      <div className="relative z-10">{children}</div>
    </Tag>
  );
}

/* ── Inline mesh gradient style for cards (no wrapper needed) ── */
export function meshGradientStyle(preset: MeshPreset = "ocean"): React.CSSProperties {
  const { bg, blobs } = presets[preset];
  const gradients = blobs
    .map((b) => `radial-gradient(circle at ${b.x} ${b.y}, ${b.color} 0%, transparent 70%)`)
    .join(", ");

  return {
    background: `${gradients}, ${bg}`,
  };
}
