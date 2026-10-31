import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { doramcap } from "./generated/protocol.js";
import { analyze, complete } from "./plan.js";
import { root, encode, fromProto, toProto, parse, textBytes } from "./model.js";
type Input = doramcap.compose.planner.v1.AnalyzeInput.$Properties;
const outputType = "doramcap.compose.planner.v1.CompleteOutput";
const modelType = root.lookupType("doramcap.compose.v1.ComposeModel");
function document(model: object, key = "app") {
  return {
    settings: { key, remove_volumes: false },
    compose_json: textBytes(JSON.stringify(model)),
  };
}
function input(
  model: object = { services: { app: { image: "app:test" } } },
): Input {
  return {
    run: "run",
    project_name: "dc-project",
    artifacts: "/runs/run",
    adopted_apps: [document(model)],
  };
}
function wired(model: object): Input {
  return {
    ...input(model),
    controller_scope: "/controller",
    owner_container_name: "experiment-owner",
    dora: {
      image: "dora:test",
      coordinator_port: 54001,
      daemon_port: 54002,
      runtime_directory: "/run/experiment",
      zenoh_overlay: "/run/experiment/zenoh.json5",
      shm_bytes: 134217728,
      memlock_bytes: 67108864,
      participant_uid: 1000,
      participant_gid: 1000,
    },
    environment: {
      node_id_variable: "DORA_NODE_ID",
      forced_values: {
        DORA_DAEMON_LOCAL_LISTEN_PORT: "54002",
        DORA_ZENOH_CONFIG_OVERLAY: "/run/experiment/zenoh.json5",
      },
      runtime_values: { DORA_DAEMON_LOCAL_LISTEN_PORT: "54002" },
    },
  };
}
function finish(request: Input) {
  const required = analyze(request);
  const output = complete({
    input: request,
    images: required.image_references!.map((reference) => ({
      reference,
      image_id: reference === "dora:test" ? "sha256:dora" : "sha256:app",
    })),
    sources: required.input_sources!.map((source) => ({
      source,
      kind: 1,
      effective_source: "/snapshots/zenoh",
    })),
    available_container_names: required.container_names,
  });
  const decoded = root
    .lookupType(outputType)
    .decode(encode(outputType, output));
  return {
    output:
      decoded as unknown as doramcap.compose.planner.v1.CompleteOutput.$Properties,
    model: fromProto(modelType, (decoded as any).compose),
  };
}

test("complete wiring matches literal Compose fixture and preserves adopted networking", () => {
  // Arrange
  const model = JSON.parse(
    readFileSync(
      new URL("./fixtures/dora-wiring.input.json", import.meta.url),
      "utf8",
    ),
  );
  const expected = JSON.parse(
    readFileSync(
      new URL("./fixtures/dora-wiring.expected.json", import.meta.url),
      "utf8",
    ),
  );
  // Act
  const result = finish(wired(model));
  // Assert
  assert.deepEqual(JSON.parse(JSON.stringify(result.model)), expected.model);
  assert.ok(
    result.output.diagnostics!.some((d) => d.message === expected.warnings[0]),
  );
});
test("merging rejects conflicts and reports unhandled top-level fields", () => {
  // Arrange
  const request = input({
    services: { app: { image: "app:test", "x-setting": { enabled: true } } },
    "future-section": {},
  });
  // Act
  const result = finish(request);
  // Assert
  assert.deepEqual(result.model.services.app["x-setting"], { enabled: true });
  assert.ok(
    result.output.diagnostics!.some((d) =>
      d.message!.includes("'future-section'"),
    ),
  );
  // Act / Assert
  request.system_apps = [
    document({ services: { app: { image: "app:test" } } }),
  ];
  assert.throws(() => analyze(request), /naming conflict/);
});
test("every resource namespace rejects duplicates; identical default networks are shared", () => {
  for (const section of [
    "services",
    "networks",
    "volumes",
    "configs",
    "secrets",
  ]) {
    // Arrange
    const request = input();
    request.adopted_apps!.push(
      document(
        {
          services: { other: { image: "app:test" } },
          [section]: { shared: {} },
        },
        "other",
      ),
    );
    (request.adopted_apps![0] as any).compose_json = textBytes(
      JSON.stringify({
        services: { app: { image: "app:test" } },
        [section]: { shared: {} },
      }),
    );
    // Act / Assert
    assert.throws(() => analyze(request), /naming conflict/);
  }
  // Arrange
  const request = input({
    services: { app: { image: "app:test" } },
    networks: { default: {} },
  });
  request.system_apps = [
    document({
      services: { system: { image: "app:test" } },
      networks: { default: {} },
    }),
  ];
  // Act / Assert
  assert.doesNotThrow(() => analyze(request));
});
test("system dependencies and graph bindings fail before host operations", () => {
  // Arrange
  const request = wired({ services: { adopted: { image: "app:test" } } });
  request.system_apps = [
    document({
      services: { monitor: { image: "app:test", depends_on: { adopted: {} } } },
    }),
  ];
  // Act / Assert
  assert.throws(() => analyze(request), /cannot depend on adopted/);
  request.system_apps = [];
  request.dora_node_ids = ["missing"];
  assert.throws(() => analyze(request), /same name/);
  request.dora_node_ids = ["adopted"];
  assert.throws(() => analyze(request), /host networking/);
});
test("reserved mounts, labels, names and malformed containers are rejected", () => {
  for (const target of [
    "/",
    "/dev",
    "/dev/shm",
    "/run/experiment/zenoh.json5",
  ]) {
    // Arrange
    const request = wired({
      services: { app: { image: "app:test", volumes: [{ target }] } },
    });
    // Act / Assert
    assert.throws(() => analyze(request), /mount overrides/);
  }
  for (const settings of [
    { labels: { "io.doramcap.controller": "foreign" } },
    { labels: { "io.doramcap.experiment-owner": "foreign" } },
    { environment: [] },
    { volumes: {} },
  ]) {
    // Arrange
    const request = wired({
      services: { app: { image: "app:test", ...settings } },
    });
    // Act / Assert
    assert.throws(() => analyze(request));
  }
  // Act / Assert
  assert.throws(
    () =>
      analyze(wired({ services: { "dora-daemon": { image: "app:test" } } })),
    /naming conflict/,
  );
});
test("fact sets must be complete, unique and consistent", () => {
  // Arrange
  const request = input();
  const image = { reference: "app:test", image_id: "sha256:pinned" };
  // Act / Assert
  for (const images of [[], [image, image], [{ ...image, reference: "other" }]])
    assert.throws(() => complete({ input: request, images }), /facts/);
  assert.doesNotThrow(() => complete({ input: request, images: [image] }));
});
test("resource ownership, completion and restart policies are applied together", () => {
  // Arrange
  const request = input({
    services: {
      app: {
        image: "app:test",
        restart: "always",
        deploy: { restart_policy: { condition: "any" } },
        depends_on: { job: { condition: "service_completed_successfully" } },
        volumes: [{ type: "volume", source: "data", target: "/data" }],
      },
      job: { image: "app:test" },
    },
    volumes: { data: {} },
  });
  // Act
  const { output, model } = finish(request);
  // Assert
  assert.equal(output.services!.job.completed, true);
  assert.equal(model.services.app.restart, "no");
  assert.equal(model.services.app.deploy.restart_policy, undefined);
  assert.equal(model.volumes.data.name, "dc-project_run_data");
  assert.equal(model.volumes.data.labels["io.doramcap.remove-volume"], "false");
  assert.equal(output.mounts![0].mount!.source, "dc-project_run_data");
});
test("image volumes, anonymous volumes, invalid replicas and unscoped resources are rejected", () => {
  for (const settings of [
    { volumes: [{ type: "volume", target: "/data" }] },
    { scale: 0 },
  ]) {
    // Arrange
    const request = input({
      services: { app: { image: "app:test", ...settings } },
    });
    // Act / Assert
    assert.throws(() => finish(request));
  }
  // Arrange
  const request = input();
  // Act / Assert
  assert.throws(
    () =>
      complete({
        input: request,
        images: [
          {
            reference: "app:test",
            image_id: "pinned",
            declared_volumes: ["/data"],
          },
        ],
      }),
    /explicit mount/,
  );
  assert.throws(
    () =>
      finish(
        input({
          services: { app: { image: "app:test" } },
          volumes: { data: { name: "foreign" } },
        }),
      ),
    /unscoped/,
  );
});
test("typed projection preserves command forms, null environment, numbers and foreign fields", () => {
  // Arrange
  const source = textBytes(
    '{"services":{"app":{"image":"app:test","command":[],"entrypoint":null,"privileged":false,"shm_size":9007199254740993,"environment":{"UNSET":null,"EMPTY":""},"ulimits":{"memlock":-1},"x-future":{"integer":9007199254740993,"decimal":1.234567890123456789,"list":[null,true,"text"]}}}}',
  );
  const typed = toProto(modelType, parse(source));
  // Act
  const restored = fromProto(
    modelType,
    modelType.decode(encode(modelType.fullName.slice(1), typed)),
  );
  // Assert
  assert.deepEqual(restored, parse(source));
});

test("typed integer fields reject overflow instead of silently truncating", () => {
  for (const source of [
    '{"services":{"app":{"image":"app:test","shm_size":18446744073709551616}}}',
    '{"services":{"app":{"image":"app:test","ulimits":{"memlock":9223372036854775808}}}}',
  ]) {
    // Arrange
    const value = parse(textBytes(source));
    // Act / Assert
    assert.throws(() => toProto(modelType, value), /range/);
  }
});

test("input snapshots preserve live directories and rewrite config/secret files", () => {
  // Arrange
  const request = input({
    services: {
      app: {
        image: "app:test",
        volumes: [
          { type: "bind", source: "/file", target: "/file", read_only: true },
          {
            type: "bind",
            source: "/directory",
            target: "/directory",
            read_only: true,
          },
          { type: "bind", source: "/live", target: "/live" },
        ],
      },
    },
    configs: { settings: { file: "/file" } },
    secrets: { token: { file: "/file" } },
  });
  // Act
  const result = complete({
    input: request,
    images: [{ reference: "app:test", image_id: "pinned" }],
    sources: [
      { source: "/file", kind: 1, effective_source: "/snapshot/file" },
      { source: "/directory", kind: 2, effective_source: "/directory" },
    ],
  });
  const model = fromProto(
    modelType,
    modelType.decode(encode(modelType.fullName.slice(1), result.compose!)),
  );
  // Assert
  assert.deepEqual(
    model.services.app.volumes.map((v: any) => v.source),
    ["/snapshot/file", "/directory", "/live"],
  );
  assert.equal(model.configs.settings.file, "/snapshot/file");
  assert.equal(model.secrets.token.file, "/snapshot/file");
});

test("identical default networks ignore property order and external volumes retain their policy", () => {
  // Arrange
  const request = input({
    services: { app: { image: "app:test" } },
    networks: { default: { driver: "bridge", internal: true } },
    volumes: {
      shared: {
        external: true,
        name: "shared",
        labels: { "io.doramcap.remove-volume": "false" },
      },
    },
  });
  request.system_apps = [
    document({
      services: { system: { image: "app:test" } },
      networks: { default: { internal: true, driver: "bridge" } },
    }),
  ];
  // Act
  const result = finish(request);
  // Assert
  assert.equal(result.model.volumes.shared.name, "shared");
  assert.equal(
    result.model.volumes.shared.labels["io.doramcap.remove-volume"],
    "false",
  );
});

test("native Compose null network attachments survive the typed output", () => {
  // Arrange
  const request = input({
    services: { app: { image: "app:test", networks: { default: null } } },
    networks: { default: { name: "dc-project_default" } },
  });
  // Act
  const result = finish(request);
  // Assert
  assert.deepEqual(result.model.services.app.networks, { default: null });
});

test("duplicate fixed container names fail before image inspection", () => {
  // Arrange
  const request = input({
    services: {
      first: { image: "app:test", container_name: "same-name" },
      second: { image: "app:test", container_name: "same-name" },
    },
  });
  // Act / Assert
  assert.throws(() => analyze(request), /container\/same-name/);
});
