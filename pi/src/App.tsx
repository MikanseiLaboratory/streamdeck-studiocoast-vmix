import { useEffect, useMemo, useRef, useState } from "react";
import {
  useGlobalSettings,
  usePluginMessage,
  useSendToPlugin,
  useSettings,
  useStreamDeck
} from "@mikanseilaboratory/streamdeck-pi-client";
import type { LiveInput, LiveInstance, VmixConfigBridge } from "./bridge";
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

const globalDefaults: GlobalSettings = {
  instances: [],
  groups: [],
  fgColor: "#f4f7fb"
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
  const [inputs, setInputs] = useState<Array<{ id: string; inputs: LiveInput[] }>>([]);
  const settingsRef = useRef(global.settings);
  const statusRef = useRef(statuses);
  const listenersRef = useRef(new Set<(instances: LiveInstance[]) => void>());
  const configWindowRef = useRef<Window | null>(null);
  const sendRef = useRef(send);
  settingsRef.current = global.settings;
  statusRef.current = statuses;
  sendRef.current = send;

  usePluginMessage((payload: { type?: string; instances?: LiveInstance[]; items?: Array<{ id: string; inputs: LiveInput[] }> }) => {
    if (payload.type === "status" && payload.instances) setStatuses(payload.instances);
    if (payload.type === "inputs" && payload.items) setInputs(payload.items);
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
          catalogs={inputs}
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
  catalogs: Array<{ id: string; inputs: LiveInput[] }>;
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
  const inputOptions = useMemo(() => {
    const ids = shared ? targeted.map((instance) => instance.id) : [activeId];
    const seen = new Map<string, LiveInput>();
    for (const item of catalogs) {
      if (!ids.includes(item.id)) continue;
      for (const input of item.inputs) seen.set(String(input.number), input);
    }
    return [...seen.values()];
  }, [catalogs, shared, targeted, activeId]);

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
  onChange
}: {
  kind: string;
  params: ActionParams;
  inputs: LiveInput[];
  onChange: (params: ActionParams) => void;
}) {
  if (kind === "shortcut") return <ShortcutFields params={params} inputs={inputs} onChange={onChange} />;
  if (kind === "raw") {
    return <TextArea label="Command" value={params.raw} onChange={(raw) => onChange({ ...params, raw })} />;
  }
  return (
    <>
      {(kind === "program" || kind === "preview" || kind === "play" || kind === "transition" || kind === "stinger" || kind === "overlay" || kind === "list" || kind === "title") &&
        showsInput(kind, params) && <InputField params={params} inputs={inputs} onChange={onChange} />}
      {(kind === "program" || kind === "preview" || kind === "transition" || kind === "stinger" || kind === "overlay") && showsMix(kind, params) && (
        <MixField value={params.mix} onChange={(mix) => onChange({ ...params, mix })} />
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

function ShortcutFields({
  params,
  inputs,
  onChange
}: {
  params: ActionParams;
  inputs: LiveInput[];
  onChange: (params: ActionParams) => void;
}) {
  const [draft, setDraft] = useState(params.functionName);
  const draftRef = useRef(draft);
  const paramsRef = useRef(params);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  draftRef.current = draft;
  paramsRef.current = params;
  useEffect(() => setDraft(params.functionName), [params.functionName]);
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    []
  );

  const commit = (name: string) => {
    if (timer.current) {
      clearTimeout(timer.current);
      timer.current = null;
    }
    const current = paramsRef.current;
    const known = SHORTCUTS.find((item) => item.Name === name);
    const allowed = new Set(known?.Parameters ?? []);
    const knownName = Boolean(known);
    const standard = ["Input", "Value", "Channel", "Mix", "Duration"];
    onChange({
      ...current,
      functionName: name,
      input: !knownName || allowed.has("Input") ? current.input : "",
      value: !knownName || allowed.has("Value") ? current.value : "",
      channel: !knownName || allowed.has("Channel") ? current.channel : "",
      mix: !knownName || allowed.has("Mix") ? current.mix : 0,
      durationMs: !knownName || allowed.has("Duration") ? current.durationMs : "",
      extra: !knownName || [...allowed].some((item) => !standard.includes(item)) ? current.extra : ""
    });
  };

  const matches = useMemo(() => {
    const term = draft.trim().toLowerCase();
    if (!term) return [];
    const found: ShortcutEntry[] = [];
    for (const item of SHORTCUTS) {
      if (item.Name.toLowerCase().includes(term)) {
        found.push(item);
        if (found.length >= MAX_SHORTCUT_SUGGESTIONS) break;
      }
    }
    return found;
  }, [draft]);
  const selected = SHORTCUTS.find((item) => item.Name === params.functionName);
  const needed = new Set(selected?.Parameters ?? []);
  const others = (selected?.Parameters ?? []).filter((item) => !["Input", "Value", "Channel", "Mix", "Duration"].includes(item));
  const showAll = !selected;

  return (
    <>
      <div className="sdpi-item">
        <div className="sdpi-item-label">Function</div>
        <input
          className="sdpi-item-value"
          type="text"
          list="shortcut-names"
          value={draft}
          onChange={(event) => {
            const name = event.target.value;
            setDraft(name);
            if (timer.current) clearTimeout(timer.current);
            timer.current = setTimeout(() => commit(name), FUNCTION_NAME_COMMIT_MS);
          }}
          onBlur={() => {
            if (draftRef.current !== params.functionName) commit(draftRef.current);
          }}
        />
        <datalist id="shortcut-names">
          {matches.map((item) => (
            <option key={item.Name} value={item.Name}>
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
      {(showAll || needed.has("Input")) && <InputField params={params} inputs={inputs} onChange={onChange} />}
      {(showAll || needed.has("Value")) && <TextField label="Value" value={params.value} onChange={(value) => onChange({ ...params, value })} />}
      {(showAll || needed.has("Channel")) && <TextField label="Channel" value={params.channel} onChange={(channel) => onChange({ ...params, channel })} />}
      {(showAll || needed.has("Mix")) && <MixField value={params.mix} onChange={(mix) => onChange({ ...params, mix })} />}
      {(showAll || needed.has("Duration")) && <TextField label="Duration" value={params.durationMs} onChange={(durationMs) => onChange({ ...params, durationMs })} />}
      {(showAll || others.length > 0) && (
        <TextField label={others.length > 0 ? others.join(", ") : "Extra"} value={params.extra} onChange={(extra) => onChange({ ...params, extra })} />
      )}
    </>
  );
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
  const listId = "vmix-inputs";
  return (
    <div className="sdpi-item">
      <div className="sdpi-item-label">Input</div>
      <input className="sdpi-item-value" type="text" list={listId} value={params.input} onChange={(event) => onChange({ ...params, input: event.target.value })} />
      <datalist id={listId}>
        {inputs.map((input) => (
          <option key={`${input.number}-${input.key}`} value={String(input.number)}>
            {input.title}
          </option>
        ))}
      </datalist>
    </div>
  );
}

function MixField({ value, onChange }: { value: number; onChange: (mix: number) => void }) {
  return (
    <div type="select" className="sdpi-item">
      <div className="sdpi-item-label">Mix</div>
      <select className="sdpi-item-value select" value={String(value === 1 ? 0 : value)} onChange={(event) => onChange(Number(event.target.value))}>
        <option value="0">Main</option>
        {Array.from({ length: 15 }, (_, index) => index + 2).map((mix) => (
          <option key={mix} value={mix}>
            Mix {mix}
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
