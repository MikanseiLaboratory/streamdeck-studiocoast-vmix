import type { GlobalSettings } from "./generated/contracts";

export type Status =
  | { kind: "disabled" }
  | { kind: "connecting" }
  | { kind: "connected"; vmixVersion?: string; edition?: string }
  | { kind: "unreachable"; message?: string }
  | { kind: string; message?: string };

export type LiveInstance = {
  id: string;
  name?: string;
  color?: string;
  status?: Status;
};

export type LiveInput = { number: number; title: string; key: string };

export type VmixConfigBridge = {
  getSettings: () => GlobalSettings;
  setSettings: (settings: GlobalSettings) => void;
  reconnect: (id?: string) => void;
  subscribe: (listener: (instances: LiveInstance[]) => void) => () => void;
};

declare global {
  interface Window {
    vmixConfig?: VmixConfigBridge;
    refreshVmixConfig?: (settings: GlobalSettings, instances: LiveInstance[]) => void;
  }
}
