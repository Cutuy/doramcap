export function StatusLabel({ label, tone = 'neutral' }: { label: string; tone?: 'active' | 'success' | 'attention' | 'failure' | 'neutral' }) {
  return <span className={`status ${tone}`}>{label}</span>;
}
