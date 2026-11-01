import { doramcap } from './generated/console';
export { routes } from './generated/transport';

export const wire = doramcap.console.v1;
export const workflow = doramcap.workflow.v1;
export const Phase = wire.Phase;
export const StepState = workflow.Event.State;
export type SystemApp = doramcap.console.v1.SystemApp;
export type Review = doramcap.console.v1.Review;
export type Step = doramcap.workflow.v1.Event;
export type Outcome = doramcap.console.v1.Outcome;
export type RunMetric = doramcap.console.v1.RunMetric;
export type Snapshot = doramcap.console.v1.Status;
