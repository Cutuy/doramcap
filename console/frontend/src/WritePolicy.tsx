import { doramcap } from './generated/console';

const { WritePolicy } = doramcap.filesystem.v1;
export const writePolicyLabel = (policy: number | null | undefined) =>
  policy === WritePolicy.WRITE_POLICY_KEEP ? 'Keep with the run'
    : policy === WritePolicy.WRITE_POLICY_DISCARD ? 'Discard' : 'Write back';

/** The saved run policy and its retained-file location. */
export function WritePolicySetting({ policy, kept }: { policy?: number | null; kept?: string | null }) {
  return <><p>Write policy: <strong>{writePolicyLabel(policy)}</strong></p>
    {kept && <p>Kept files: <span className="mono break">{kept}</span></p>}</>;
}
