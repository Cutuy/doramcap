import Long from "long";
import type { doramcap } from "./generated/protocol.js";
import {
  root,
  parse,
  object,
  numeric,
  integer,
  toProto,
  type Document,
} from "./model.js";
type P = doramcap.compose.planner.v1.AnalyzeInput.$Properties;
type Diagnostic = doramcap.compose.v1.Diagnostic.$Properties;
const RUN = "io.doramcap.run",
  ARTIFACTS = "io.doramcap.artifacts";
const CONTROLLER = "io.doramcap.controller",
  OWNER = "io.doramcap.experiment-owner";
const DAEMON = "dora-daemon",
  COORDINATOR = "dora-coordinator";
const warn = (message: string): Diagnostic => ({
  operation: "compose planner",
  kind: "warning",
  message,
});
const entries = (o: any) => Object.entries(o ?? {}) as [string, Document][];
function labels(resource: Document, values: Record<string, string>) {
  const target = object(
    (resource.labels ??= Object.create(null)),
    "resource labels",
  );
  for (const [key, value] of Object.entries(values)) {
    if (Object.hasOwn(target, key))
      throw new Error(`reserved resource ownership label ${key}`);
    target[key] = value;
  }
}
function canonical(value: any): string {
  return JSON.stringify(value, (_key, item) =>
    item &&
    typeof item === "object" &&
    !Array.isArray(item) &&
    !JSON.isRawJSON(item)
      ? Object.fromEntries(
          Object.entries(item).sort(([a], [b]) => a.localeCompare(b)),
        )
      : item,
  );
}
function merge(model: Document, part: Document, diagnostics: Diagnostic[]) {
  for (const kind of [
    "services",
    "networks",
    "volumes",
    "configs",
    "secrets",
  ]) {
    if (!Object.hasOwn(part, kind)) continue;
    const target = (model[kind] ??= Object.create(null));
    for (const [key, value] of entries(object(part[kind], kind))) {
      if (Object.hasOwn(target, key)) {
        if (
          kind === "networks" &&
          key === "default" &&
          canonical(target[key]) === canonical(value)
        )
          continue;
        throw new Error(`Compose naming conflict: ${kind}/${key}`);
      }
      target[key] = value;
    }
  }
  for (const key of Object.keys(part).filter(
    (k) =>
      ![
        "services",
        "networks",
        "volumes",
        "configs",
        "secrets",
        "name",
        "version",
      ].includes(k),
  ))
    diagnostics.push(
      warn(`unsupported Compose top-level field '${key}' is ignored`),
    );
}
function runtimeServices(input: P): Document {
  const d = input.dora!;
  const bind = (source: string, read_only?: boolean) => ({
    type: "bind",
    source,
    target: source,
    ...(read_only === undefined ? {} : { read_only }),
  });
  const service = (command: string[]) => ({
    image: d.image,
    network_mode: "host",
    stop_signal: "SIGINT",
    ulimits: {
      memlock: {
        soft: integer(d.memlock_bytes),
        hard: integer(d.memlock_bytes),
      },
    },
    user: `${d.participant_uid}:${d.participant_gid}`,
    working_dir: d.runtime_directory,
    environment: {
      HOME: d.runtime_directory,
      ...input.environment!.runtime_values,
    },
    entrypoint: ["/usr/local/bin/dora"],
    command,
    volumes: [bind(d.runtime_directory!), bind(d.zenoh_overlay!, true)],
    ipc: `service:${DAEMON}`,
  });
  const coordinator = service([
    "coordinator",
    "--interface",
    "127.0.0.1",
    "--port",
    String(d.coordinator_port),
    "--store",
    "memory",
  ]);
  if (d.graph) coordinator.volumes.push(bind(d.graph, true));
  return {
    [COORDINATOR]: coordinator,
    [DAEMON]: {
      ...service([
        "daemon",
        "--coordinator-port",
        String(d.coordinator_port),
        "--local-listen-port",
        String(d.daemon_port),
        "--zenoh-config-overlay",
        d.zenoh_overlay!,
      ]),
      ipc: "shareable",
      shm_size: integer(d.shm_bytes),
      container_name: input.owner_container_name,
      labels: { [OWNER]: "true" },
    },
  };
}
// Assemble the complete document before discovering any host-side requirements.
function assemble(input: P) {
  const model: Document = { services: Object.create(null) };
  const diagnostics: Diagnostic[] = [],
    system: string[] = [],
    adopted: string[] = [],
    completed: string[] = [];
  const apps = [...(input.system_apps ?? []), ...(input.adopted_apps ?? [])];
  // Merge app documents while collecting their startup and completion groups.
  for (const [documents, group] of [
    [input.system_apps ?? [], system],
    [input.adopted_apps ?? [], adopted],
  ] as const) {
    for (const document of documents) {
      const part = parse(document.compose_json!);
      const services = object(part.services, "Compose services");
      group.push(...Object.keys(services));
      completed.push(...(document.settings?.completed_services ?? []));
      for (const [, volume] of entries(part.volumes).filter(
        ([, v]) => v.external !== true,
      ))
        labels(volume, {
          "io.doramcap.remove-volume": String(
            document.settings?.remove_volumes ?? false,
          ),
        });
      merge(model, part, diagnostics);
    }
  }
  // Wire application IPC before adding the coordinator and daemon services.
  if (input.dora) {
    if (
      !input.environment?.node_id_variable ||
      !input.dora.runtime_directory ||
      input.dora.participant_uid == null ||
      input.dora.participant_gid == null
    )
      throw new Error("resolved Dora settings missing");
    const d = input.dora;
    for (const [name, service] of entries(model.services)) {
      object(service, "Compose service");
      if (service.labels?.[OWNER] !== undefined)
        throw new Error("reserved experiment-owner label");
      service.ipc = `service:${DAEMON}`;
      delete service.shm_size;
      object((service.ulimits ??= {}), "ulimits").memlock = {
        soft: integer(d.memlock_bytes),
        hard: integer(d.memlock_bytes),
      };
      const env = object(
        (service.environment ??= Object.create(null)),
        "Compose environment",
      );
      const key = input.environment.node_id_variable;
      if (!Object.hasOwn(env, key)) env[key] = name;
      Object.assign(env, input.environment.forced_values);
      const mounts = (service.volumes ??= []);
      if (!Array.isArray(mounts)) throw new Error("invalid Compose volumes");
      if (
        mounts.some((m) =>
          ["/", "/dev", "/dev/shm", d.zenoh_overlay].includes(m.target),
        )
      )
        throw new Error(
          "app mount overrides shared IPC or reserved Dora settings",
        );
      mounts.push({
        type: "bind",
        source: d.zenoh_overlay,
        target: d.zenoh_overlay,
        read_only: true,
      });
      if (service.network_mode !== "host")
        diagnostics.push(
          warn(
            `${name}: IPC silos: Compose networking is preserved; Dora 1.1 nodes need host networking to reach the daemon on localhost. IPC and /dev/shm are shared.`,
          ),
        );
    }
    merge(model, { services: runtimeServices(input) }, diagnostics);
    for (const kind of ["services", "networks"])
      for (const [, resource] of entries(model[kind]).filter(
        ([, r]) => r.external !== true,
      ))
        labels(resource, { [CONTROLLER]: input.controller_scope! });
    for (const name of input.dora_node_ids ?? []) {
      if (!Object.hasOwn(model.services, name))
        throw new Error(
          `Dora node ${name} requires a Compose service with the same name`,
        );
      if (model.services[name].network_mode !== "host")
        throw new Error(`Dora node ${name} requires host networking`);
    }
  }
  // Reject dependencies that would bypass the system-first startup barrier.
  for (const name of system)
    if (
      Object.keys(model.services[name].depends_on ?? {}).some((dep) =>
        adopted.includes(dep),
      )
    )
      throw new Error(
        "system apps cannot depend on adopted apps during startup",
      );
  if (!Object.keys(model.services).length)
    throw new Error("selection contains no services");
  return {
    model,
    diagnostics,
    system,
    adopted,
    completed,
    remove_volumes: apps.every((a) => a.settings?.remove_volumes === true),
  };
}
function requirements(
  model: Document,
): doramcap.compose.planner.v1.AnalyzeOutput.$Properties {
  const images = new Set<string>(),
    sources = new Set<string>(),
    names = new Set<string>();
  for (const [name, service] of entries(model.services)) {
    if (typeof service.image !== "string" || !service.image)
      throw new Error(
        `${name}: prepare a named local image before the experiment`,
      );
    images.add(service.image);
    if (service.container_name) {
      if (names.has(service.container_name))
        throw new Error(
          `Compose naming conflict: container/${service.container_name}`,
        );
      names.add(service.container_name);
    }
    for (const mount of service.volumes ?? [])
      if (mount.type === "bind" && mount.read_only === true)
        sources.add(mount.source);
  }
  for (const kind of ["configs", "secrets"])
    for (const [, value] of entries(model[kind]))
      if (value.file) sources.add(value.file);
  return {
    image_references: [...images].sort(),
    input_sources: [...sources].sort(),
    container_names: [...names].sort(),
  };
}
export function analyze(
  input: P,
): doramcap.compose.planner.v1.AnalyzeOutput.$Properties {
  const { model, diagnostics } = assemble(input);
  return { ...requirements(model), diagnostics };
}
function facts<T>(
  required: string[],
  provided: T[],
  key: (v: T) => string,
): Map<string, T> {
  const result = new Map(provided.map((v) => [key(v), v]));
  if (
    provided.length !== result.size ||
    result.size !== required.length ||
    required.some((k) => !result.has(k))
  )
    throw new Error("missing, duplicate or unexpected planner facts");
  return result;
}
// Apply host facts in one pass; all Docker field interpretation stays in this module.
export function complete(
  request: doramcap.compose.planner.v1.CompleteInput.$Properties,
): doramcap.compose.planner.v1.CompleteOutput.$Properties {
  const input = request.input!;
  const { model, diagnostics, system, adopted, completed, remove_volumes } =
    assemble(input);
  const required = requirements(model);
  // Require one host fact for each discovered image, source and container name.
  const images = facts(
    required.image_references!,
    request.images ?? [],
    (v) => v.reference!,
  );
  const sources = facts(
    required.input_sources!,
    request.sources ?? [],
    (v) => v.source!,
  );
  facts(
    required.container_names!,
    request.available_container_names ?? [],
    (v) => v,
  );
  const snapshot = (source: string) => {
    const fact = sources.get(source)!;
    if (
      ![1, 2].includes(Number(fact.kind)) ||
      !fact.effective_source?.startsWith("/") ||
      (fact.kind === 2 && fact.effective_source !== source)
    )
      throw new Error("invalid input source fact");
    return fact.effective_source;
  };
  // Scope resource names and attach experiment ownership.
  for (const kind of ["networks", "volumes"])
    for (const [key, resource] of entries(model[kind])) {
      if (resource.external === true) {
        diagnostics.push(
          warn(`external ${kind}/${key} retained; reset not verified`),
        );
        continue;
      }
      if (resource.name && resource.name !== `${input.project_name}_${key}`)
        throw new Error(
          `unscoped ${kind}/${key}: explicit resource name requires external ownership`,
        );
      if (kind === "volumes")
        resource.name = `${input.project_name}_${input.run}_${key}`;
      labels(resource, { [RUN]: input.run!, [ARTIFACTS]: input.artifacts! });
    }
  // Recognize completion dependencies before assigning service lifecycle policies.
  for (const [, service] of entries(model.services))
    for (const [name, dep] of entries(service.depends_on))
      if (dep.condition === "service_completed_successfully")
        completed.push(name);
  if (completed.some((name) => !Object.hasOwn(model.services, name)))
    throw new Error("completed_services refers to an unselected service");
  const services: Record<
    string,
    doramcap.compose.v1.ComposeService.$Properties
  > = Object.create(null);
  const mounts: doramcap.compose.planner.v1.ServiceMount.$Properties[] = [];
  const mountType = root.lookupType("doramcap.compose.v1.ComposeMount");
  // Pin images, freeze restart policy and apply snapshotted input paths.
  for (const [name, service] of entries(model.services)) {
    const image = images.get(service.image)!;
    if (!image.image_id) throw new Error("Engine omitted image ID");
    const volumes: Document[] = service.volumes ?? [];
    if (volumes.some((v) => v.type === "volume" && !Object.hasOwn(v, "source")))
      throw new Error(
        `${name}: anonymous volumes need explicit named-volume retention settings`,
      );
    for (const target of image.declared_volumes ?? [])
      if (!volumes.some((v) => v.target === target))
        throw new Error(
          `${name}: image volume ${target} needs an explicit mount for ownership/retention`,
        );
    const replicas = numeric(service.scale ?? service.deploy?.replicas ?? 1);
    if (
      !/^\d+$/.test(replicas) ||
      BigInt(replicas) === 0n ||
      BigInt(replicas) > 18446744073709551615n
    )
      throw new Error(`${name}: invalid or zero replicas`);
    for (const [condition, text] of [
      [service.network_mode === "host", "host networking; ports are shared"],
      [service.privileged === true, "privileged; containment is cooperative"],
      [
        Object.hasOwn(service, "volumes"),
        "bind/external input contents are live; container cleanup does not reset them",
      ],
      [!Object.hasOwn(service, "healthcheck"), "process-level readiness only"],
    ] as const)
      if (condition) diagnostics.push(warn(`${name}: ${text}`));
    if (service.restart && service.restart !== "no")
      diagnostics.push(
        warn(
          `${name}: restart policy overridden to no for experiment ownership`,
        ),
      );
    service.restart = "no";
    if (service.deploy?.restart_policy !== undefined) {
      delete service.deploy.restart_policy;
      diagnostics.push(
        warn(`${name}: deploy restart policy removed for experiment ownership`),
      );
    }
    for (const key of ["build", "profiles", "pull_policy"]) delete service[key];
    service.image = image.image_id;
    labels(service, { [RUN]: input.run!, [ARTIFACTS]: input.artifacts! });
    for (const volume of volumes) {
      if (volume.type === "bind" && volume.read_only === true)
        volume.source = snapshot(volume.source);
      if (["bind", "volume"].includes(volume.type)) {
        const mount = { ...volume };
        if (mount.type === "volume") {
          const resource = model.volumes?.[mount.source];
          if (!resource?.name) throw new Error("volume has no resolved name");
          mount.source = resource.name;
        }
        mounts.push({ service: name, mount: toProto(mountType, mount) });
      }
    }
    services[name] = {
      image_id: image.image_id,
      replicas: Long.fromString(replicas, true),
      completed: completed.includes(name),
    };
  }
  for (const kind of ["configs", "secrets"])
    for (const [, item] of entries(model[kind]))
      if (item.file) {
        if (sources.get(item.file)?.kind !== 1)
          throw new Error("Compose config/secret source must be a file");
        item.file = snapshot(item.file);
      }
  return {
    compose: toProto(
      root.lookupType("doramcap.compose.v1.ComposeModel"),
      model,
    ),
    services,
    system_services: system,
    adopted_services: adopted,
    mounts,
    diagnostics,
    remove_volumes,
  };
}
