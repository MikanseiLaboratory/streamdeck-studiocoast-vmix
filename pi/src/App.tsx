import { useEffect, useMemo, useRef, useState } from "react";
import {
  useGlobalSettings,
  usePluginMessage,
  useSendToPlugin,
  useSettings,
  useStreamDeck
} from "@mikanseilaboratory/streamdeck-pi-client";
import type { LiveCatalog, LiveInput, LiveInstance, VmixConfigBridge } from "./bridge";
import type { ActionParams, ActionSettings, GlobalSettings, InstanceConfig, TargetGroup, TargetSelector } from "./generated/contracts";
import shortcuts from "./generated/shortcuts.json";

const FUNCTION_NAME_COMMIT_MS = 400;
const MAX_SHORTCUT_SUGGESTIONS = 50;

type ShortcutEntry = { Name: string; Description: string; Parameters: string[] | null };
const SHORTCUTS = shortcuts as ShortcutEntry[];

const emptyParams = (): ActionParams => ({
  input: "",
  mix: 0,
  effect: "Cut",
  durationMs: "",
  overlay: 1,
  overlayMode: "toggle",
  stinger: 1,
  replayAction: "play",
  channel: "",
  audioTarget: "input",
  bus: "A",
  listAction: "next",
  index: "",
  titleAction: "settext",
  value: "",
  selectedName: "",
  functionName: "",
  extra: "",
  raw: "",
  step: 1
});

const actionDefaults: ActionSettings = {
  common: { target: { kind: "all" }, sharedParams: true },
  advanced: { longPress: false, longPressMs: 0 },
  shared: emptyParams(),
  params: {}
};

const localhostInstance = (): InstanceConfig => ({
  id: "localhost",
  name: "Localhost",
  host: "127.0.0.1",
  port: 8099,
  color: "#4c8dff",
  enabled: true,
  xmlIntervalMs: 2000
});

const globalDefaults: GlobalSettings = {
  instances: [localhostInstance()],
  groups: [],
  fgColor: "#f4f7fb",
  seeded: false
};

const CONFIG_WINDOW = "vmix-config";
const TRANSITION_BUTTONS = ["Transition1", "Transition2", "Transition3", "Transition4"];
const EFFECTS = ["Cut", "Fade", "Merge", ...TRANSITION_BUTTONS];

export function App() {
  const deck = useStreamDeck();
  const kind = deck.actionInfo?.action.split(".").at(-1) ?? "program";
  const global = useGlobalSettings<GlobalSettings>(globalDefaults);
  const action = useSettings<ActionSettings>(actionDefaults);
  const send = useSendToPlugin();
  const [statuses, setStatuses] = useState<LiveInstance[]>([]);
  const [catalogs, setCatalogs] = useState<LiveCatalog[]>([]);
  const settingsRef = useRef(global.settings);
  const statusRef = useRef(statuses);
  const listenersRef = useRef(new Set<(instances: LiveInstance[]) => void>());
  const configWindowRef = useRef<Window | null>(null);
  const sendRef = useRef(send);
  settingsRef.current = global.settings;
  statusRef.current = statuses;
  sendRef.current = send;

  usePluginMessage((payload: { type?: string; instances?: LiveInstance[]; items?: LiveCatalog[] }) => {
    if (payload.type === "status" && payload.instances) setStatuses(payload.instances);
    if (payload.type === "inputs" && payload.items) setCatalogs(payload.items);
  });

  useEffect(() => {
    const bridge: VmixConfigBridge = {
      getSettings: () => settingsRef.current,
      setSettings: (next) => global.setSettings(next),
      reconnect: (id) => {
        const ids = id ? [id] : settingsRef.current.instances.map((instance) => instance.id);
        for (const item of ids) sendRef.current({ type: "reconnect", id: item });
      },
      subscribe: (listener) => {
        listenersRef.current.add(listener);
        listener(statusRef.current);
        return () => listenersRef.current.delete(listener);
      }
    };
    window.vmixConfig = bridge;
    return () => {
      if (window.vmixConfig === bridge) delete window.vmixConfig;
    };
  }, [global.setSettings]);

  useEffect(() => {
    for (const listener of listenersRef.current) listener(statuses);
    const child = configWindowRef.current;
    if (child && !child.closed && child.opener === window) {
      child.refreshVmixConfig?.(settingsRef.current, statuses);
    }
  }, [statuses, global.settings]);

  useEffect(() => {
    send({ type: "ready" });
  }, [send]);

  useEffect(() => {
    return deck.subscribe("didReceiveGlobalSettings", (payload) => {
      if (!payload || typeof payload !== "object" || Array.isArray(payload)) return;
      sendRef.current({ type: "globalSettings", settings: payload });
    });
  }, [deck]);

  const instances = global.settings.instances ?? [];
  const groups = global.settings.groups ?? [];
  const target = action.settings.common?.target ?? { kind: "all" as const };
  const hasFields = kind !== "fadetoblack" && kind !== "recording" && kind !== "streaming" && kind !== "external" && kind !== "multicorder" && kind !== "fullscreen";

  const openConfig = () => {
    const features = "width=760,height=840";
    const existing = window.open("", CONFIG_WINDOW);
    if (!existing) return;
    let owned = false;
    try {
      owned = existing.opener === window;
    } catch {
      owned = false;
    }
    if (!owned) {
      existing.close();
      configWindowRef.current = window.open("./configuration.html", CONFIG_WINDOW, features);
      return;
    }
    const href = existing.location.href;
    if (href.includes("configuration.html")) {
      existing.focus();
      existing.refreshVmixConfig?.(settingsRef.current, statusRef.current);
      configWindowRef.current = existing;
      return;
    }
    existing.location.href = new URL("./configuration.html", window.location.href).href;
    configWindowRef.current = existing;
  };

  return (
    <div className="sdpi-wrapper">
      <div className="sdpi-heading">Target</div>
      <TargetPicker
        instances={instances}
        groups={groups}
        statuses={statuses}
        value={target}
        onChange={(next) =>
          action.setSettings((previous) => ({
            ...previous,
            common: { ...previous.common, target: next }
          }))
        }
      />
      {hasFields && (
        <CheckRow
          label="Parameters"
          checked={action.settings.common?.sharedParams !== false}
          text="Same values for every target"
          onChange={(checked) =>
            action.setSettings((previous) => ({
              ...previous,
              common: { ...previous.common, sharedParams: checked }
            }))
          }
        />
      )}
      {hasFields && (
        <Params
          kind={kind}
          shared={action.settings.common?.sharedParams !== false}
          instances={instances}
          target={target}
          groups={groups}
          settings={action.settings}
          setSettings={action.setSettings}
          catalogs={catalogs}
        />
      )}
      <div className="sdpi-heading">Connections</div>
      <div className="sdpi-item">
        <div className="sdpi-item-label">vMix</div>
        <button className="sdpi-item-value" type="button" onClick={openConfig}>
          Manage instances
        </button>
      </div>
    </div>
  );
}

function TargetPicker({
  instances,
  groups,
  statuses,
  value,
  onChange
}: {
  instances: InstanceConfig[];
  groups: TargetGroup[];
  statuses: LiveInstance[];
  value: TargetSelector;
  onChange: (value: TargetSelector) => void;
}) {
  const [preferMultiple, setPreferMultiple] = useState(value.kind === "instances" && value.ids.length !== 1);
  useEffect(() => {
    if (value.kind !== "instances") setPreferMultiple(false);
  }, [value]);
  const mode = preferMultiple ? "multiple" : targetMode(value);
  const selected = value.kind === "instances" ? value.ids : [];
  return (
    <>
      <div type="select" className="sdpi-item">
        <div className="sdpi-item-label">Send to</div>
        <select
          className="sdpi-item-value select"
          value={mode}
          onChange={(event) => {
            const next = event.target.value;
            setPreferMultiple(next === "multiple");
            onChange(selectorFromMode(next, selected));
          }}
        >
          <option value="all">All enabled</option>
          {groups.map((group) => (
            <option key={group.id} value={`group:${group.id}`}>
              Group: {group.name || "Untitled"}
            </option>
          ))}
          {instances.map((instance) => (
            <option key={instance.id} value={`instance:${instance.id}`}>
              {instance.name || "vMix"}
              {instance.enabled ? "" : " (disabled)"}
            </option>
          ))}
          <option value="multiple">Multiple instances</option>
        </select>
      </div>
      {mode === "multiple" && (
        <div type="checkbox" className="sdpi-item targets">
          <div className="sdpi-item-label">Instances</div>
          <div className="sdpi-item-value">
            {instances.length === 0 && <span>No instances yet</span>}
            {instances.map((instance) => {
              const id = `target-${instance.id}`;
              const on = selected.includes(instance.id);
              const status = statuses.find((item) => item.id === instance.id)?.status?.kind;
              return (
                <div className="sdpi-item-child" key={instance.id}>
                  <input
                    id={id}
                    type="checkbox"
                    checked={on}
                    onChange={() => {
                      const ids = on ? selected.filter((item) => item !== instance.id) : [...selected, instance.id];
                      onChange({ kind: "instances", ids });
                    }}
                  />
                  <label htmlFor={id}>
                    <span></span>
                    <i className={`swatch status-dot ${status ?? ""}`} style={{ background: statusColor(status, instance.color) }} />
                    {instance.name || "vMix"}
                  </label>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </>
  );
}

function CheckRow({
  label,
  checked,
  text,
  onChange
}: {
  label: string;
  checked: boolean;
  text: string;
  onChange: (checked: boolean) => void;
}) {
  const id = `check-${label.replace(/\s+/g, "-").toLowerCase()}`;
  return (
    <div type="checkbox" className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <input id={id} className="sdpi-item-value" type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <label htmlFor={id}>
        <span></span>
        {text}
      </label>
    </div>
  );
}

function Params({
  kind,
  shared,
  instances,
  target,
  groups,
  settings,
  setSettings,
  catalogs
}: {
  kind: string;
  shared: boolean;
  instances: InstanceConfig[];
  target: TargetSelector;
  groups: TargetGroup[];
  settings: ActionSettings;
  setSettings: (next: ActionSettings | ((previous: ActionSettings) => ActionSettings)) => void;
  catalogs: LiveCatalog[];
}) {
  const targeted = useMemo(() => instancesForTarget(instances, groups, target), [instances, groups, target]);
  const [selectedId, setSelectedId] = useState(targeted[0]?.id ?? "");
  const activeId = targeted.some((instance) => instance.id === selectedId) ? selectedId : targeted[0]?.id ?? "";
  const editors = shared
    ? [{ id: "shared", name: "Shared", params: settings.shared ?? emptyParams() }]
    : targeted
        .filter((instance) => instance.id === activeId)
        .map((instance) => ({
          id: instance.id,
          name: instance.name,
          params: settings.params?.[instance.id] ?? settings.shared ?? emptyParams()
        }));
  const catalogIds = useMemo(
    () => catalogIdsFor(catalogs, shared ? targeted.map((instance) => instance.id) : [activeId]),
    [catalogs, shared, targeted, activeId]
  );
  const inputOptions = useMemo(() => inputsFor(catalogs, catalogIds), [catalogs, catalogIds]);
  const mixOptions = useMemo(() => mixesFor(catalogs, catalogIds), [catalogs, catalogIds]);

  const write = (id: string, params: ActionParams) => {
    setSettings((previous) => {
      if (id === "shared") return { ...previous, shared: params };
      return { ...previous, params: { ...previous.params, [id]: params } };
    });
  };

  return (
    <>
      <div className="sdpi-heading">Action</div>
      {!shared && (
        <div type="select" className="sdpi-item">
          <div className="sdpi-item-label">Instance</div>
          <select className="sdpi-item-value select" value={activeId} onChange={(event) => setSelectedId(event.target.value)}>
            {targeted.map((instance) => (
              <option key={instance.id} value={instance.id}>
                {instance.name || "vMix"}
              </option>
            ))}
          </select>
        </div>
      )}
      {editors.map((editor) => (
        <ActionFields
          key={editor.id}
          kind={kind}
          params={editor.params}
          inputs={inputOptions}
          mixes={mixOptions}
          onChange={(params) => write(editor.id, params)}
        />
      ))}
    </>
  );
}

function ActionFields({
  kind,
  params,
  inputs,
  mixes,
  onChange
}: {
  kind: string;
  params: ActionParams;
  inputs: LiveInput[];
  mixes: number[];
  onChange: (params: ActionParams) => void;
}) {
  if (kind === "shortcut") return <ShortcutFields params={params} onChange={onChange} />;
  if (kind === "raw") {
    return <TextArea label="Command" value={params.raw} onChange={(raw) => onChange({ ...params, raw })} />;
  }
  return (
    <>
      {(kind === "program" || kind === "preview" || kind === "play" || kind === "transition" || kind === "stinger" || kind === "overlay" || kind === "list" || kind === "title") &&
        showsInput(kind, params) && <InputField params={params} inputs={inputs} onChange={onChange} />}
      {(kind === "program" || kind === "preview" || kind === "transition" || kind === "stinger" || kind === "overlay") && showsMix(kind, params) && (
        <MixField value={params.mix} mixes={mixes} onChange={(mix) => onChange({ ...params, mix })} />
      )}
      {kind === "transition" && (
        <>
          <TextField label="Effect" value={params.effect} listId="transition-effects" options={EFFECTS} onChange={(effect) => onChange({ ...params, effect })} />
          {!TRANSITION_BUTTONS.includes(params.effect.trim()) && (
            <TextField label="Duration" value={params.durationMs} onChange={(durationMs) => onChange({ ...params, durationMs })} />
          )}
        </>
      )}
      {kind === "stinger" && <NumberSelect label="Stinger" value={params.stinger} max={8} onChange={(stinger) => onChange({ ...params, stinger })} />}
      {kind === "overlay" && (
        <>
          <NumberSelect label="Overlay" value={params.overlay} max={8} onChange={(overlay) => onChange({ ...params, overlay })} />
          <SelectField
            label="Mode"
            value={params.overlayMode}
            options={["toggle", "in", "out", "off", "zoom", "last", "preview"]}
            onChange={(overlayMode) => onChange({ ...params, overlayMode })}
          />
        </>
      )}
      {kind === "replay" && (
        <>
          <SelectField
            label="Action"
            value={params.replayAction}
            options={["play", "pause", "markin", "markout", "markinout", "channel"]}
            onChange={(replayAction) => onChange({ ...params, replayAction })}
          />
          <SelectField label="Channel" value={params.channel || "a"} options={["a", "b"]} onChange={(channel) => onChange({ ...params, channel })} />
        </>
      )}
      {(kind === "mute" || kind === "solo" || kind === "bussend" || kind === "volume") && (
        <AudioFields kind={kind} params={params} inputs={inputs} onChange={onChange} />
      )}
      {kind === "list" && (
        <>
          <SelectField label="Action" value={params.listAction} options={["next", "previous", "select"]} onChange={(listAction) => onChange({ ...params, listAction })} />
          {params.listAction === "select" && <TextField label="Index" value={params.index} onChange={(index) => onChange({ ...params, index })} />}
        </>
      )}
      {kind === "title" && <TitleFields params={params} onChange={onChange} />}
    </>
  );
}

function showsInput(kind: string, params: ActionParams) {
  if (kind === "transition") return !TRANSITION_BUTTONS.includes(params.effect.trim());
  if (kind === "overlay") return params.overlayMode === "toggle" || params.overlayMode === "in" || params.overlayMode === "preview" || params.overlayMode === "";
  return true;
}

function showsMix(kind: string, params: ActionParams) {
  if (kind === "transition") return !TRANSITION_BUTTONS.includes(params.effect.trim());
  if (kind === "overlay") return params.overlayMode === "toggle" || params.overlayMode === "in" || params.overlayMode === "last" || params.overlayMode === "";
  return true;
}

function AudioFields({
  kind,
  params,
  inputs,
  onChange
}: {
  kind: string;
  params: ActionParams;
  inputs: LiveInput[];
  onChange: (params: ActionParams) => void;
}) {
  const buses = kind === "bussend" ? ["A", "B", "C", "D", "E", "F", "G", "M"] : ["A", "B", "C", "D", "E", "F", "G"];
  return (
    <>
      {kind !== "bussend" && (
        <SelectField
          label="Target"
          value={params.audioTarget}
          options={["input", "master", "bus"]}
          onChange={(audioTarget) => onChange({ ...params, audioTarget })}
        />
      )}
      {(kind === "bussend" || params.audioTarget === "input" || params.audioTarget === "") && (
        <InputField params={params} inputs={inputs} onChange={onChange} />
      )}
      {(kind === "bussend" || params.audioTarget === "bus") && (
        <SelectField label="Bus" value={params.bus} options={buses} onChange={(bus) => onChange({ ...params, bus })} />
      )}
      {kind === "volume" && (
        <div className="sdpi-item">
          <div className="sdpi-item-label">Step</div>
          <input
            className="sdpi-item-value"
            type="number"
            min={1}
            max={100}
            step={1}
            value={params.step}
            onChange={(event) => onChange({ ...params, step: Number(event.target.value) })}
          />
        </div>
      )}
    </>
  );
}

function TitleFields({ params, onChange }: { params: ActionParams; onChange: (params: ActionParams) => void }) {
  return (
    <>
      <SelectField
        label="Action"
        value={params.titleAction}
        options={["settext", "visible", "setimage"]}
        onChange={(titleAction) => onChange({ ...params, titleAction })}
      />
      {params.titleAction !== "visible" && <TextField label="Value" value={params.value} onChange={(value) => onChange({ ...params, value })} />}
      <TextField label="Selected name" value={params.selectedName} onChange={(selectedName) => onChange({ ...params, selectedName })} />
    </>
  );
}

function ShortcutFields({ params, onChange }: { params: ActionParams; onChange: (params: ActionParams) => void }) {
  const shown = shortcutText(params);
  const [draft, setDraft] = useState(shown);
  const draftRef = useRef(draft);
  const paramsRef = useRef(params);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  draftRef.current = draft;
  paramsRef.current = params;
  useEffect(() => setDraft(shown), [shown]);
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    []
  );

  const commit = (line: string) => {
    if (timer.current) {
      clearTimeout(timer.current);
      timer.current = null;
    }
    const current = paramsRef.current;
    onChange({
      ...current,
      functionName: line.trim(),
      input: "",
      value: "",
      channel: "",
      mix: 0,
      durationMs: "",
      extra: ""
    });
  };

  const term = shortcutFunctionName(draft).toLowerCase();
  const matches = useMemo(() => {
    if (!term) return [];
    const found: ShortcutEntry[] = [];
    for (const item of SHORTCUTS) {
      if (item.Name.toLowerCase().includes(term)) {
        found.push(item);
        if (found.length >= MAX_SHORTCUT_SUGGESTIONS) break;
      }
    }
    return found;
  }, [term]);
  const selected = SHORTCUTS.find((item) => item.Name.toLowerCase() === term);

  return (
    <>
      <div className="sdpi-item">
        <div className="sdpi-item-label">Shortcut</div>
        <input
          className="sdpi-item-value"
          type="text"
          list="shortcut-names"
          spellCheck={false}
          placeholder="Function=SetText&Input=1&Value=hello"
          value={draft}
          onChange={(event) => {
            const line = event.target.value;
            setDraft(line);
            if (timer.current) clearTimeout(timer.current);
            timer.current = setTimeout(() => commit(line), FUNCTION_NAME_COMMIT_MS);
          }}
          onBlur={() => {
            if (draftRef.current.trim() !== shortcutText(paramsRef.current)) commit(draftRef.current);
          }}
        />
        <datalist id="shortcut-names">
          {matches.map((item) => (
            <option key={item.Name} value={`Function=${item.Name}`}>
              {item.Description}
            </option>
          ))}
        </datalist>
      </div>
      {selected && (
        <p className="caution">
          {selected.Description}
          {selected.Parameters && selected.Parameters.length > 0 ? ` · ${selected.Parameters.join(", ")}` : ""}
        </p>
      )}
    </>
  );
}

function shortcutText(params: ActionParams) {
  const name = params.functionName.trim();
  if (shortcutLineIsComplete(name)) return name;
  const parts: string[] = [];
  if (name) parts.push(`Function=${name}`);
  if (params.input.trim()) parts.push(`Input=${params.input.trim()}`);
  if (params.value) parts.push(`Value=${params.value}`);
  if (params.channel.trim()) parts.push(`Channel=${params.channel.trim()}`);
  if (params.mix > 0) parts.push(`Mix=${params.mix}`);
  if (params.durationMs.trim()) parts.push(`Duration=${params.durationMs.trim()}`);
  const extra = params.extra.trim().replace(/^&/, "");
  if (extra) parts.push(extra);
  return parts.join("&");
}

function shortcutLineIsComplete(value: string) {
  const lower = value.toLowerCase();
  return (
    lower.startsWith("http://") ||
    lower.startsWith("https://") ||
    lower.startsWith("function=") ||
    lower.startsWith("function ") ||
    value.includes("&") ||
    value.includes("?")
  );
}

function shortcutFunctionName(line: string) {
  const text = line.trim();
  const fromQuery = text.match(/(?:^|[?&])Function=([^&]+)/i);
  if (fromQuery?.[1]) return decodeURIComponent(fromQuery[1]);
  const tcp = text.match(/^FUNCTION\s+(\S+)/i);
  if (tcp?.[1]) return tcp[1];
  const head = text.split(/[&?\s]/)[0] ?? "";
  if (head && !head.includes("=")) return head;
  return "";
}

function InputField({
  params,
  inputs,
  onChange
}: {
  params: ActionParams;
  inputs: LiveInput[];
  onChange: (params: ActionParams) => void;
}) {
  const current = params.input.trim();
  const known = inputs.some(
    (input) => String(input.number) === current || input.title === current || input.key === current
  );
  return (
    <div type="select" className="sdpi-item">
      <div className="sdpi-item-label">Input</div>
      <select className="sdpi-item-value select" value={current} onChange={(event) => onChange({ ...params, input: event.target.value })}>
        <option value="">{inputs.length === 0 ? "Waiting for inputs" : "Select input"}</option>
        {current !== "" && !known && <option value={current}>{current}</option>}
        {inputs.map((input) => (
          <option key={`${input.number}-${input.key}`} value={String(input.number)}>
            {inputLabel(input)}
          </option>
        ))}
      </select>
    </div>
  );
}

function MixField({ value, mixes, onChange }: { value: number; mixes: number[]; onChange: (mix: number) => void }) {
  const current = value === 1 ? 0 : value;
  const options = mixes.includes(current) ? mixes : [...mixes, current].sort((left, right) => left - right);
  return (
    <div type="select" className="sdpi-item">
      <div className="sdpi-item-label">Mix</div>
      <select className="sdpi-item-value select" value={String(current)} onChange={(event) => onChange(Number(event.target.value))}>
        {options.map((mix) => (
          <option key={mix} value={mix}>
            {mix === 0 ? "Main" : `Mix ${mix}`}
          </option>
        ))}
      </select>
    </div>
  );
}

function NumberSelect({ label, value, max, onChange }: { label: string; value: number; max: number; onChange: (value: number) => void }) {
  const current = Math.min(max, Math.max(1, value || 1));
  return (
    <div type="select" className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <select className="sdpi-item-value select" value={String(current)} onChange={(event) => onChange(Number(event.target.value))}>
        {Array.from({ length: max }, (_, index) => index + 1).map((item) => (
          <option key={item} value={item}>
            {item}
          </option>
        ))}
      </select>
    </div>
  );
}

function SelectField({
  label,
  value,
  options,
  onChange
}: {
  label: string;
  value: string;
  options: string[];
  onChange: (value: string) => void;
}) {
  return (
    <div type="select" className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <select className="sdpi-item-value select" value={value} onChange={(event) => onChange(event.target.value)}>
        {options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    </div>
  );
}

function TextField({
  label,
  value,
  onChange,
  listId,
  options
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  listId?: string;
  options?: string[];
}) {
  return (
    <div className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <input className="sdpi-item-value" type="text" list={listId} value={value} onChange={(event) => onChange(event.target.value)} />
      {listId && options && (
        <datalist id={listId}>
          {options.map((option) => (
            <option key={option} value={option} />
          ))}
        </datalist>
      )}
    </div>
  );
}

function TextArea({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <textarea className="sdpi-item-value" rows={4} value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

function targetMode(value: TargetSelector) {
  if (value.kind === "all") return "all";
  if (value.kind === "group") return `group:${value.id}`;
  if (value.ids.length === 1) return `instance:${value.ids[0]}`;
  return "multiple";
}

function selectorFromMode(mode: string, selected: string[]): TargetSelector {
  if (mode === "all") return { kind: "all" };
  if (mode === "multiple") return { kind: "instances", ids: selected };
  if (mode.startsWith("group:")) return { kind: "group", id: mode.slice("group:".length) };
  if (mode.startsWith("instance:")) return { kind: "instances", ids: [mode.slice("instance:".length)] };
  return { kind: "all" };
}

function catalogIdsFor(catalogs: LiveCatalog[], ids: string[]) {
  const wanted = ids.filter((id) => id);
  const matched = catalogs.filter((item) => wanted.includes(item.id) && item.inputs.length > 0);
  if (matched.length > 0) return matched.map((item) => item.id);
  const available = catalogs.filter((item) => item.inputs.length > 0);
  if (available.length > 0) return available.map((item) => item.id);
  return wanted;
}

function inputsFor(catalogs: LiveCatalog[], ids: string[]) {
  const byNumber = new Map<number, LiveInput>();
  for (const item of catalogs) {
    if (!ids.includes(item.id)) continue;
    for (const input of item.inputs) {
      const existing = byNumber.get(input.number);
      if (!existing) {
        byNumber.set(input.number, input);
        continue;
      }
      if (input.title && existing.title !== input.title && !existing.title.split(" / ").includes(input.title)) {
        byNumber.set(input.number, { ...existing, title: `${existing.title} / ${input.title}` });
      }
    }
  }
  return [...byNumber.values()].sort((left, right) => left.number - right.number);
}

function mixesFor(catalogs: LiveCatalog[], ids: string[]) {
  const known = catalogs.filter((item) => ids.includes(item.id) && item.mixes.length > 0);
  if (known.length === 0) return [0];
  const present = new Set<number>();
  for (const item of known) {
    for (const mix of item.mixes) present.add(mix === 1 ? 0 : mix);
  }
  if (!present.has(0)) present.add(0);
  return [...present].sort((left, right) => left - right);
}

function inputLabel(input: LiveInput) {
  const title = input.title || input.shortTitle || input.key;
  return title ? `${input.number}: ${title}` : String(input.number);
}

function instancesForTarget(instances: InstanceConfig[], groups: TargetGroup[], target: TargetSelector) {
  if (target.kind === "group") {
    const group = groups.find((item) => item.id === target.id);
    return instances.filter((instance) => group?.members.includes(instance.id));
  }
  if (target.kind === "instances") return instances.filter((instance) => target.ids.includes(instance.id));
  return instances.filter((instance) => instance.enabled);
}

function statusColor(kind: string | undefined, fallback: string) {
  switch (kind) {
    case "connected":
      return "#3cba7a";
    case "connecting":
      return "#e2b15a";
    case "disabled":
      return "#6d7890";
    case "unreachable":
      return "#8a4a3a";
    default:
      return fallback;
  }
}
