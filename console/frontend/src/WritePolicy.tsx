import { doramcap } from './generated/console';

const { WritePolicy } = doramcap.filesystem.v1;
export const writePolicyLabel = (policy: number | null | undefined) =>
  policy === WritePolicy.WRITE_POLICY_KEEP ? 'Keep with the run'
    : policy === WritePolicy.WRITE_POLICY_DISCARD ? 'Discard'
      : policy == null || policy === WritePolicy.WRITE_POLICY_UNSPECIFIED || policy === WritePolicy.WRITE_POLICY_WRITE_BACK ? 'Write back' : 'Unknown';

const options = [
  { policy: WritePolicy.WRITE_POLICY_KEEP, description: 'Files stay with this run for download. The host folder is unchanged.' },
  { policy: WritePolicy.WRITE_POLICY_WRITE_BACK, description: 'Files are copied into the host folder after the run.' },
  { policy: WritePolicy.WRITE_POLICY_DISCARD, description: 'Files are listed, then removed. The host folder is unchanged.' },
];

/** Review's per-run choice; changing it requires a fresh prepared review. */
export function WritePolicyChoice({ policy, choose, busy }: { policy?: number | null; choose: (policy: doramcap.filesystem.v1.WritePolicy) => void; busy?: boolean }) {
  return <fieldset disabled={busy}><legend>Write policy</legend>{options.map(option => <label className="checkbox" key={option.policy}>
    <input type="radio" name="write-policy" value={option.policy} checked={policy === option.policy} onChange={() => choose(option.policy)}/>
    <span><strong>{writePolicyLabel(option.policy)}</strong><br/>{option.description}</span>
  </label>)}</fieldset>;
}

/** The saved run policy and intended retained-file destination. */
export function WritePolicySetting({ policy, kept }: { policy?: number | null; kept?: string | null }) {
  return <><p>Write policy: <strong>{writePolicyLabel(policy)}</strong></p>
    {kept && <p>Kept files destination: <span className="mono break">{kept}</span></p>}</>;
}
