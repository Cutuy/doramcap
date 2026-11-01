import { doramcap } from './generated/console';
import { contract } from './generated/transport';

export const wire = doramcap.console.v1;
export const workflow = doramcap.workflow.v1;
export const Phase = wire.Phase;
export const StepState = workflow.Event.State;
export const routes = contract(wire);
export type Service = doramcap.console.v1.Service;
export type SystemApp = doramcap.console.v1.SystemApp;
export type RecordingOutput = doramcap.console.v1.RecordingOutput;
export type Review = doramcap.console.v1.Review;
export type Step = doramcap.workflow.v1.Event;
export type Outcome = doramcap.console.v1.Outcome;
export type RunMetric = doramcap.console.v1.RunMetric;
export type Snapshot = doramcap.console.v1.Status;
