export function EmptyState({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <div className="border border-dashed border-border bg-card-alt px-5 py-8 text-center">
      <div className="text-[13px] font-semibold">{title}</div>
      <p className="mt-2 text-[12px] leading-relaxed text-text-secondary">{description}</p>
    </div>
  );
}

