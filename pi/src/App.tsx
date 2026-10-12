import { useEffect, useMemo, useRef, useState } from "react";
import {
  useGlobalSettings,
  usePluginMessage,
  useSendToPlugin,
  useSettings,
  useStreamDeck
} from "@mikanseilaboratory/streamdeck-pi-client";
import type { LiveCatalog, LiveInput, LiveInstance, VmixConfigBridge } from "./bridge";
import type { ActionParams, ActionSettings, GlobalSettings, InstanceConfig, ShortcutCall, TargetGroup, TargetSelector } from "./generated/contracts";
import shortcuts from "./generated/shortcuts.json";

type ShortcutEntry = { Name: string; Description: string; Parameters: string[] | null; Category?: string };
const SHORTCUTS = shortcuts as ShortcutEntry[];

function categoryForShortcutKind(kind: string) {
  if (!kind.startsWith("shortcut-")) return undefined;
  const slug = kind.slice("shortcut-".length);
  return SHORTCUTS.find((item) => (item.Category ?? "").toLowerCase() === slug)?.Category;
}

const emptyParams = (): ActionParams => ({
  input: "",
  useInputNumber: false,
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
  step: 1,
  rateMode: "rate",
  mixerMode: "bus",
  programMode: "cutDirect",
  activeColor: "",
  release: emptyCall(),
  touch: emptyCall(),
  rotateUp: emptyCall(),
  rotateDown: emptyCall()
});

const emptyCall = (): ShortcutCall => ({
  functionName: "",
  input: "",
  value: "",
  channel: "",
  mix: 0,
  durationMs: "",
  selectedName: "",
  extra: ""
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
  xmlIntervalMs: 100
});

const globalDefaults: GlobalSettings = {
  instances: [localhostInstance()],
  groups: [],
  fgColor: "#f4f7fb",
  seeded: false
};

const CONFIG_WINDOW = "vmix-config";
const TRANSITION_BUTTONS = ["Transition1", "Transition2", "Transition3", "Transition4"];
const STINGERS = ["Stinger1", "Stinger2", "Stinger3", "Stinger4", "Stinger5", "Stinger6", "Stinger7", "Stinger8"];
const EFFECTS = ["Cut", "Fade", "Merge", ...TRANSITION_BUTTONS, ...STINGERS];
const DIALS = ["volume", "replayjog", "gain", "headphones", "mixer", "rate", "replayspeed", "position"];
const TOGGLES = ["overlay", "fadetoblack", "recording", "streaming", "external", "multicorder", "fullscreen", "replay", "mute", "solo", "bussend", "play", "list"];

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
    const retry = window.setTimeout(() => send({ type: "ready" }), 250);
    return () => window.clearTimeout(retry);
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
  const hasFields = true;

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
  const dynamic = useMemo(() => dynamicFor(catalogs, catalogIds), [catalogs, catalogIds]);

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
          dynamic={dynamic}
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
  dynamic,
  onChange
}: {
  kind: string;
  params: ActionParams;
  inputs: LiveInput[];
  mixes: number[];
  dynamic: string[];
  onChange: (params: ActionParams) => void;
}) {
  const release = (
    <ShortcutFields
      label="On release"
      call={params.release ?? emptyCall()}
      inputs={inputs}
      mixes={mixes}
      dynamic={dynamic}
      onChange={(release) => onChange({ ...params, release })}
    />
  );
  if (kind === "shortcut" || kind.startsWith("shortcut-")) {
    return (
      <>
        <ShortcutFields
          label="Shortcut"
          category={categoryForShortcutKind(kind)}
          call={callFromParams(params)}
          inputs={inputs}
          mixes={mixes}
          dynamic={dynamic}
          onChange={(call) => onChange(applyCall(params, call))}
        />
        {release}
      </>
    );
  }
  const dial = (
    <DialFunctions params={params} inputs={inputs} mixes={mixes} dynamic={dynamic} onChange={onChange} />
  );
  if (kind === "replayjog" || kind === "replayspeed") {
    return (
      <>
        <SelectField label="Channel" value={params.channel || "a"} options={["a", "b"]} onChange={(channel) => onChange({ ...params, channel })} />
        <StepField params={params} onChange={onChange} min={kind === "replayspeed" ? 0.05 : 1} max={kind === "replayspeed" ? 1 : 100} increment={kind === "replayspeed" ? 0.05 : 1} />
        {dial}
      </>
    );
  }
  if (kind === "gain") {
    return (
      <>
        <InputField params={params} inputs={inputs} dynamic={dynamic} onChange={onChange} />
        <StepField params={params} onChange={onChange} min={0.5} max={6} increment={0.5} />
        {dial}
      </>
    );
  }
  if (kind === "headphones") {
    return (
      <>
        <StepField params={params} onChange={onChange} min={1} max={100} increment={1} />
        {dial}
      </>
    );
  }
  if (kind === "mixer") {
    return (
      <>
        <InputField params={params} inputs={inputs} dynamic={dynamic} onChange={onChange} />
        <SelectField label="Mixer" value={params.mixerMode} options={["bus", "channel"]} onChange={(mixerMode) => onChange({ ...params, mixerMode })} />
        {params.mixerMode === "channel" ? (
          <NumberSelect label="Channel" value={Number(params.index) || 1} max={16} onChange={(index) => onChange({ ...params, index: String(index) })} />
        ) : (
          <SelectField label="Bus" value={params.bus} options={["A", "B", "C", "D", "E", "F", "G", "M"]} onChange={(bus) => onChange({ ...params, bus })} />
        )}
        <StepField params={params} onChange={onChange} min={1} max={100} increment={1} />
        {dial}
      </>
    );
  }
  if (kind === "rate") {
    return (
      <>
        <InputField params={params} inputs={inputs} dynamic={dynamic} onChange={onChange} />
        <SelectField label="Mode" value={params.rateMode} options={["rate", "slow"]} onChange={(rateMode) => onChange({ ...params, rateMode })} />
        <StepField params={params} onChange={onChange} min={0.05} max={1} increment={0.05} />
        {dial}
      </>
    );
  }
  if (kind === "position") {
    return (
      <>
        <InputField params={params} inputs={inputs} dynamic={dynamic} onChange={onChange} />
        <StepField params={params} onChange={onChange} min={100} max={10000} increment={100} />
        {dial}
      </>
    );
  }
  if (kind === "raw") {
    return (
      <>
        <TextField label="Function" value={params.raw} onChange={(raw) => onChange({ ...params, raw })} />
        <p className="shortcut-note">Type Play&amp;Input=1. The FUNCTION word is added when sending.</p>
        {release}
      </>
    );
  }
  return (
    <>
      {(kind === "program" || kind === "preview" || kind === "play" || kind === "transition" || kind === "stinger" || kind === "overlay" || kind === "list" || kind === "title") &&
        showsInput(kind, params) && <InputField params={params} inputs={inputs} dynamic={dynamic} onChange={onChange} />}
      {(kind === "program" || kind === "preview" || kind === "transition" || kind === "stinger" || kind === "overlay") && showsMix(kind, params) && (
        <MixField
          value={params.mix}
          mixes={mixes}
          onChange={(mix) =>
            onChange({
              ...params,
              mix,
              programMode: kind === "program" && mix !== 0 && (params.programMode === "cutDirect" || !params.programMode) ? "cut" : params.programMode
            })
          }
        />
      )}
      {kind === "program" && (
        <SelectField
          label="Switch"
          value={params.mix === 0 ? params.programMode || "cutDirect" : params.programMode === "activeInput" ? "activeInput" : "cut"}
          options={params.mix === 0 ? ["cutDirect", "cut", "activeInput"] : ["cut", "activeInput"]}
          labels={params.mix === 0 ? ["CutDirect", "Cut", "ActiveInput"] : ["Cut", "ActiveInput"]}
          onChange={(programMode) => onChange({ ...params, programMode })}
        />
      )}
      {kind === "transition" && (
        <>
          <TextField label="Effect" value={params.effect} listId="transition-effects" options={EFFECTS} onChange={(effect) => onChange({ ...params, effect })} />
          {!hidesDuration(params.effect) && (
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
            options={["play", "pause", "markin", "markout", "markinout", "channel", "startstop", "next", "previous"]}
            labels={["Play", "Pause", "Mark In", "Mark Out", "Mark In/Out", "Channel", "Start/Stop Recording", "Select Next", "Select Previous"]}
            onChange={(replayAction) => onChange({ ...params, replayAction })}
          />
          {params.replayAction !== "startstop" && (
            <SelectField label="Channel" value={params.channel || "a"} options={["a", "b"]} onChange={(channel) => onChange({ ...params, channel })} />
          )}
        </>
      )}
      {(kind === "mute" || kind === "solo" || kind === "bussend" || kind === "volume") && (
        <AudioFields kind={kind} params={params} inputs={inputs} dynamic={dynamic} onChange={onChange} />
      )}
      {kind === "list" && (
        <>
          <SelectField label="Action" value={params.listAction} options={["next", "previous", "select"]} onChange={(listAction) => onChange({ ...params, listAction })} />
          {params.listAction === "select" && <TextField label="Index" value={params.index} onChange={(index) => onChange({ ...params, index })} />}
        </>
      )}
      {kind === "title" && <TitleFields params={params} onChange={onChange} />}
      {TOGGLES.includes(kind) && (
        <TextField label="Active color" value={params.activeColor} onChange={(activeColor) => onChange({ ...params, activeColor })} />
      )}
      {kind === "volume" && dial}
      {!DIALS.includes(kind) && release}
    </>
  );
}

function hidesDuration(effect: string) {
  const name = effect.trim();
  return TRANSITION_BUTTONS.includes(name) || STINGERS.includes(name);
}

function DialFunctions({
  params,
  inputs,
  mixes,
  dynamic,
  onChange
}: {
  params: ActionParams;
  inputs: LiveInput[];
  mixes: number[];
  dynamic: string[];
  onChange: (params: ActionParams) => void;
}) {
  return (
    <>
      <ShortcutFields
        label="On touch"
        call={params.touch ?? emptyCall()}
        inputs={inputs}
        mixes={mixes}
        dynamic={dynamic}
        onChange={(touch) => onChange({ ...params, touch })}
      />
      <ShortcutFields
        label="Rotate right"
        call={params.rotateUp ?? emptyCall()}
        inputs={inputs}
        mixes={mixes}
        dynamic={dynamic}
        onChange={(rotateUp) => onChange({ ...params, rotateUp })}
      />
      <ShortcutFields
        label="Rotate left"
        call={params.rotateDown ?? emptyCall()}
        inputs={inputs}
        mixes={mixes}
        dynamic={dynamic}
        onChange={(rotateDown) => onChange({ ...params, rotateDown })}
      />
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
  dynamic,
  onChange
}: {
  kind: string;
  params: ActionParams;
  inputs: LiveInput[];
  dynamic: string[];
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
        <InputField params={params} inputs={inputs} dynamic={dynamic} onChange={onChange} />
      )}
      {(kind === "bussend" || params.audioTarget === "bus") && (
        <SelectField label="Bus" value={params.bus} options={buses} onChange={(bus) => onChange({ ...params, bus })} />
      )}
      {kind === "volume" && <StepField params={params} onChange={onChange} min={1} max={100} increment={1} />}
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
  label,
  category,
  call,
  inputs,
  mixes,
  dynamic,
  onChange
}: {
  label: string;
  category?: string;
  call: ShortcutCall;
  inputs: LiveInput[];
  mixes: number[];
  dynamic: string[];
  onChange: (call: ShortcutCall) => void;
}) {
  const migrated = migrateShortcutCall(call);
  useEffect(() => {
    if (migrated) onChange(migrated);
  }, [migrated, onChange]);
  const current = migrated ?? call;
  const entries = category ? SHORTCUTS.filter((item) => item.Category === category) : SHORTCUTS;
  const selected = entries.find((item) => item.Name === current.functionName);
  const parameters = selected?.Parameters ?? [];
  const missing = parameters.filter((name) => shortcutParamEmpty(name, current));
  const note = formatShortcutNote(selected?.Description ?? "");
  return (
    <>
      <div type="select" className="sdpi-item">
        <div className="sdpi-item-label">{label}</div>
        <select
          className="sdpi-item-value select"
          value={entries.some((item) => item.Name === current.functionName) ? current.functionName : ""}
          onChange={(event) => onChange({ ...emptyCall(), functionName: event.target.value })}
        >
          <option value="">None</option>
          {entries.map((item) => (
            <option key={item.Name} value={item.Name}>
              {item.Name}
            </option>
          ))}
        </select>
      </div>
      {note && <p className="shortcut-note">{note}</p>}
      {parameters.map((name) => (
        <ShortcutParam
          key={name}
          name={name}
          call={current}
          inputs={inputs}
          mixes={mixes}
          dynamic={dynamic}
          onChange={onChange}
        />
      ))}
      {current.functionName && missing.length > 0 && (
        <p className="shortcut-note">{`Missing: ${missing.join(", ")}`}</p>
      )}
    </>
  );
}

function ShortcutParam({
  name,
  call,
  inputs,
  mixes,
  dynamic,
  onChange
}: {
  name: string;
  call: ShortcutCall;
  inputs: LiveInput[];
  mixes: number[];
  dynamic: string[];
  onChange: (call: ShortcutCall) => void;
}) {
  if (name === "Input") {
    return (
      <InputField
        params={callAsParams(call)}
        inputs={inputs}
        dynamic={dynamic}
        allowNumber={false}
        onChange={(params) => onChange({ ...call, input: params.input })}
      />
    );
  }
  if (name === "Mix") {
    return <MixField value={call.mix} mixes={mixes} onChange={(mix) => onChange({ ...call, mix })} />;
  }
  if (name === "Duration") {
    return <TextField label="Duration" value={call.durationMs} onChange={(durationMs) => onChange({ ...call, durationMs })} />;
  }
  if (name === "Value") {
    return <TextField label="Value" value={call.value} onChange={(value) => onChange({ ...call, value })} />;
  }
  if (name === "Channel") {
    return <TextField label="Channel" value={call.channel} onChange={(channel) => onChange({ ...call, channel })} />;
  }
  if (name === "SelectedName") {
    return <TextField label="Selected name" value={call.selectedName} onChange={(selectedName) => onChange({ ...call, selectedName })} />;
  }
  const extra = extraMap(call.extra);
  return (
    <TextField
      label={name}
      value={extra[name] ?? ""}
      onChange={(value) => onChange({ ...call, extra: writeExtra({ ...extra, [name]: value }) })}
    />
  );
}

function callAsParams(call: ShortcutCall): ActionParams {
  return { ...emptyParams(), input: call.input };
}

function callFromParams(params: ActionParams): ShortcutCall {
  return {
    functionName: params.functionName,
    input: params.input,
    value: params.value,
    channel: params.channel,
    mix: params.mix,
    durationMs: params.durationMs,
    selectedName: params.selectedName,
    extra: params.extra
  };
}

function applyCall(params: ActionParams, call: ShortcutCall): ActionParams {
  return {
    ...params,
    functionName: call.functionName,
    input: call.input,
    value: call.value,
    channel: call.channel,
    mix: call.mix,
    durationMs: call.durationMs,
    selectedName: call.selectedName,
    extra: call.extra
  };
}

function shortcutParamEmpty(name: string, call: ShortcutCall) {
  if (name === "Input") return call.input.trim() === "";
  if (name === "Mix") return false;
  if (name === "Duration") return call.durationMs.trim() === "";
  if (name === "Value") return call.value.trim() === "";
  if (name === "Channel") return call.channel.trim() === "";
  if (name === "SelectedName") return call.selectedName.trim() === "";
  return (extraMap(call.extra)[name] ?? "").trim() === "";
}

function formatShortcutNote(description: string) {
  return description
    .split(/ {2,}/)
    .map((part) => part.trim())
    .filter(Boolean)
    .join("\n");
}

function extraMap(extra: string) {
  const map: Record<string, string> = {};
  for (const piece of extra.split("&")) {
    if (!piece) continue;
    const index = piece.indexOf("=");
    if (index < 0) continue;
    const key = decodeURIComponent(piece.slice(0, index));
    map[key] = decodeURIComponent(piece.slice(index + 1));
  }
  return map;
}

function writeExtra(map: Record<string, string>) {
  return Object.entries(map)
    .filter(([, value]) => value !== "")
    .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
    .join("&");
}

function migrateShortcutCall(call: ShortcutCall): ShortcutCall | null {
  const line = call.functionName.trim();
  if (!shortcutLineIsComplete(line)) return null;
  const body = line.includes("://") ? line.slice(line.indexOf("?") + 1) : line.replace(/^function\s+/i, "");
  const next = emptyCall();
  for (const piece of body.split("&")) {
    const index = piece.indexOf("=");
    if (index < 0) {
      if (!next.functionName) next.functionName = decodeURIComponent(piece.trim());
      continue;
    }
    const key = piece.slice(0, index).trim();
    const value = decodeURIComponent(piece.slice(index + 1).trim());
    if (key.toLowerCase() === "function") next.functionName = value;
    else if (key.toLowerCase() === "input") next.input = value;
    else if (key.toLowerCase() === "value") next.value = value;
    else if (key.toLowerCase() === "channel") next.channel = value;
    else if (key.toLowerCase() === "duration") next.durationMs = value;
    else if (key.toLowerCase() === "selectedname") next.selectedName = value;
    else if (key.toLowerCase() === "mix") {
      const tcp = Number(value);
      next.mix = Number.isFinite(tcp) ? tcp + 1 : 0;
    } else {
      const extra = extraMap(next.extra);
      extra[key] = value;
      next.extra = writeExtra(extra);
    }
  }
  return next.functionName ? next : null;
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

function InputField({
  params,
  inputs,
  dynamic = [],
  allowNumber = true,
  onChange
}: {
  params: ActionParams;
  inputs: LiveInput[];
  dynamic?: string[];
  allowNumber?: boolean;
  onChange: (params: ActionParams) => void;
}) {
  const useNumber = params.useInputNumber === true;
  const current = params.input.trim();
  const paramsRef = useRef(params);
  paramsRef.current = params;
  useEffect(() => {
    if (isSpecialInput(current)) return;
    const match = inputs.find((input) => input.key === current || String(input.number) === current);
    if (!match) return;
    const next = inputToken(match, useNumber);
    if (next === current) return;
    onChange({ ...paramsRef.current, input: next });
  }, [inputs, current, useNumber, onChange]);
  const known = isSpecialInput(current) || inputs.some(
    (input) => inputToken(input, useNumber) === current || input.key === current || String(input.number) === current
  );
  const specials = [
    { value: "Active", label: "Active" },
    { value: "Preview", label: "Preview" },
    ...[1, 2, 3, 4].map((slot) => ({
      value: `Dynamic${slot}`,
      label: dynamicLabel(slot, dynamic[slot - 1] ?? "", inputs)
    }))
  ];
  return (
    <>
      <div type="select" className="sdpi-item">
        <div className="sdpi-item-label">Input</div>
        <select className="sdpi-item-value select" value={current} onChange={(event) => onChange({ ...params, input: event.target.value })}>
          <option value="">{inputs.length === 0 ? "Waiting for inputs" : "Select input"}</option>
          {specials.map((item) => (
            <option key={item.value} value={item.value}>
              {item.label}
            </option>
          ))}
          {current !== "" && !known && <option value={current}>{current}</option>}
          {inputs.map((input) => (
            <option key={`${input.number}-${input.key}`} value={inputToken(input, useNumber)}>
              {inputLabel(input)}
            </option>
          ))}
        </select>
      </div>
      {allowNumber && <CheckRow
        label="Number"
        checked={useNumber}
        text="Use number instead"
        onChange={(checked) => {
          const match = inputs.find((input) => input.key === current || String(input.number) === current);
          onChange({
            ...params,
            useInputNumber: checked,
            input: match ? inputToken(match, checked) : current
          });
        }}
      />}
    </>
  );
}

function isSpecialInput(value: string) {
  return value === "Active" || value === "Preview" || /^Dynamic[1-4]$/.test(value);
}

function dynamicLabel(slot: number, raw: string, inputs: LiveInput[]) {
  const token = raw.trim();
  if (!token) return `Dynamic ${slot}`;
  const match = inputs.find((input) => String(input.number) === token || input.key === token || input.title === token);
  return match ? `Dynamic ${slot}: ${inputLabel(match)}` : `Dynamic ${slot}: ${token}`;
}

function inputToken(input: LiveInput, useNumber: boolean) {
  if (useNumber || !input.key) return String(input.number);
  return input.key;
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

function StepField({
  params,
  onChange,
  min,
  max,
  increment
}: {
  params: ActionParams;
  onChange: (params: ActionParams) => void;
  min: number;
  max: number;
  increment: number;
}) {
  return (
    <div className="sdpi-item">
      <div className="sdpi-item-label">Step</div>
      <input
        className="sdpi-item-value"
        type="number"
        min={min}
        max={max}
        step={increment}
        value={params.step}
        onChange={(event) => onChange({ ...params, step: Number(event.target.value) })}
      />
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
  labels,
  onChange
}: {
  label: string;
  value: string;
  options: string[];
  labels?: string[];
  onChange: (value: string) => void;
}) {
  return (
    <div type="select" className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <select className="sdpi-item-value select" value={value} onChange={(event) => onChange(event.target.value)}>
        {options.map((option, index) => (
          <option key={option} value={option}>
            {labels?.[index] ?? option}
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

function dynamicFor(catalogs: LiveCatalog[], ids: string[]) {
  const known = catalogs.find((item) => ids.includes(item.id) && item.dynamic && item.dynamic.some((slot) => slot.trim() !== ""));
  return known?.dynamic ?? ["", "", "", ""];
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
