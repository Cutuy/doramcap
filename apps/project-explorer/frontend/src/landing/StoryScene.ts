import * as THREE from 'three';
import { CSS2DObject, CSS2DRenderer } from 'three/addons/renderers/CSS2DRenderer.js';
import { RoundedBoxGeometry } from 'three/addons/geometries/RoundedBoxGeometry.js';
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import type { Flow, Integration } from '../overview-model';
import { pairOffset, type NodeState, type StackNode, type StackView, type Tier } from '../stack';

// Plain sRGB keeps scene colors identical to the CSS tokens.
THREE.ColorManagement.enabled = false;

const tiers: Tier[] = ['compute', 'platform', 'foundation', 'applications'];
const tierTokens: Record<Tier, string> = { applications: '--tier-apps', foundation: '--tier-core', platform: '--tier-platform', compute: '--tier-compute' };
const tierLabels: Record<Tier, string> = { applications: 'Your applications', foundation: 'doramcap', platform: 'Platform', compute: 'Compute' };
const SLAB_W = 17.5, SLAB_D = 7.4, TILE_H = .42;
const HERO_GAP = 1.25, STORY_GAP = 3.1, TRAIL = 10, COMETS = 3, STEPS = 64;
const damp = (current: number, target: number, rate: number, dt: number) => current + (target - current) * (1 - Math.exp(-rate * dt));

interface Place { x: number; z: number; w: number; d: number; }
interface Block { node: StackNode; mesh: THREE.Mesh; body: THREE.MeshStandardMaterial; edge: THREE.LineBasicMaterial; label: HTMLDivElement; glow: number; alpha: number; line: number; delay: number; }
interface Trail { flow: Flow; curve: THREE.CubicBezierCurve3; line: THREE.Line; comets: THREE.Points; phase: number; }
export interface SceneEvents { hover(node: StackNode | null, x: number, y: number): void; select(node: StackNode | null): void; }

/** Place blocks per tier; the foundation uses two rows so doramcap gets the most room. */
function layout(nodes: StackNode[], integrations: Integration[]): Map<string, Place> {
  const places = new Map<string, Place>();
  const row = (ids: string[], spacing: number, w: number, z = 0, d = 1.5) => ids.forEach((id, i) => places.set(id, { x: (i - (ids.length - 1) / 2) * spacing, z, w, d }));
  const of = (t: Tier) => nodes.filter(n => n.tier === t).map(n => n.id);
  row(of('applications'), 3.9, 3.1);
  const foundation = of('foundation');
  row(foundation.slice(0, 4), 4.05, 3.3, -1.55);
  row(foundation.slice(4), 4.05, 3.3, 1.55);
  row([...integrations.filter(i => i.group === 'execution'), ...integrations.filter(i => i.group !== 'execution')].map(i => i.id), 2.7, 2.3);
  row(of('compute'), 5.2, 4.3);
  return places;
}

const cometVertex = `
attribute float aSize; attribute float aAlpha; uniform float uScale; varying float vAlpha;
void main() { vAlpha = aAlpha; vec4 mv = modelViewMatrix * vec4(position, 1.0); gl_PointSize = aSize * uScale / -mv.z; gl_Position = projectionMatrix * mv; }`;
const cometFragment = `
uniform vec3 uColor; uniform float uFade; uniform float uLight; varying float vAlpha;
void main() { float d = length(gl_PointCoord - 0.5); float a = smoothstep(0.5, 0.0, d); gl_FragColor = vec4(mix(uColor * (1.0 + a), uColor, uLight), a * vAlpha * uFade); }`;

/** Cinematic platform stack: hero orbit at progress 0, isometric story view from progress 1.
 * Owns its renderer, labels and loop; callers drive it with setProgress/setView/setSelected and must dispose().
 * Throws if WebGL is unavailable.
 */
export class StoryScene {
  private renderer: THREE.WebGLRenderer;
  private labels = new CSS2DRenderer();
  private composer: EffectComposer;
  private bloom: UnrealBloomPass;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(30, 1, .1, 400);
  private clock = new THREE.Clock();
  private slabs: { tier: Tier; group: THREE.Group; fill: THREE.MeshBasicMaterial; edge: THREE.LineBasicMaterial; tag: HTMLDivElement }[] = [];
  private blocks: Block[] = [];
  private trails: Trail[] = [];
  private dust: THREE.Points;
  private palette: Record<string, THREE.Color> = {};
  private probe = document.createElement('span');
  private resizer: ResizeObserver;
  private frame = 0;
  private running = false;
  private progress = 0;
  private gap = HERO_GAP;
  private view: StackView = { states: {}, flows: [] };
  private selected: string | null = null;
  private hovered: StackNode | null = null;
  private pointer = new THREE.Vector2();
  private parallax = new THREE.Vector2();
  private orbit = 0;
  private target = new THREE.Vector3(0, 3.6, 0);
  private radius = 46;
  private polar = 1.18;
  private fade = 0;
  private intro = 0;
  private sweep = 1;
  private sweepTimer = 0;
  private sweepPlane: THREE.Mesh<THREE.PlaneGeometry, THREE.ShaderMaterial>;
  private down: [number, number] | null = null;
  private reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  private light = false;
  private hemi = new THREE.HemisphereLight(0xb8c8ff, 0x101418, .9);
  private key = new THREE.DirectionalLight(0xffffff, 1.1);
  private grid: THREE.GridHelper | null = null;
  private scheme = matchMedia('(prefers-color-scheme: dark)');
  private themeWatch = new MutationObserver(() => this.applyTheme());
  private raycaster = new THREE.Raycaster();

  constructor(private host: HTMLElement, nodes: StackNode[], integrations: Integration[], private events: SceneEvents) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
    this.renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
    this.renderer.setPixelRatio(Math.min(devicePixelRatio, 1.75));
    host.append(this.renderer.domElement, this.labels.domElement, this.probe);
    this.labels.domElement.className = 'scene-labels';
    this.probe.hidden = true;
    this.readPalette();

    this.scene.fog = new THREE.FogExp2(this.palette.bg, .014);
    this.key.position.set(-8, 22, 14);
    this.scene.add(this.hemi, this.key);

    // Slow-rising dust gives depth without competing with the stack.
    const dust = new Float32Array(1400 * 3);
    for (let i = 0; i < 1400; i++) dust.set([(Math.random() - .5) * 90, Math.random() * 34 - 4, (Math.random() - .5) * 90], i * 3);
    this.dust = new THREE.Points(new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(dust, 3)),
      new THREE.PointsMaterial({ color: this.palette.muted, size: .07, transparent: true, opacity: .45, depthWrite: false }));
    this.scene.add(this.dust);

    const places = layout(nodes, integrations);
    for (const tier of tiers) {
      const group = new THREE.Group(); this.scene.add(group);
      const fill = new THREE.MeshBasicMaterial({ color: this.palette[tier], transparent: true, opacity: .05, depthWrite: false, side: THREE.DoubleSide });
      const plate = new THREE.Mesh(new THREE.PlaneGeometry(SLAB_W, SLAB_D), fill); plate.rotation.x = -Math.PI / 2; group.add(plate);
      const edge = new THREE.LineBasicMaterial({ color: this.palette[tier], transparent: true, opacity: .45 });
      group.add(new THREE.LineSegments(new THREE.EdgesGeometry(new THREE.PlaneGeometry(SLAB_W, SLAB_D)).rotateX(-Math.PI / 2), edge));
      const tag = document.createElement('div'); tag.className = `scene-tier tier-${tier}`; tag.textContent = tierLabels[tier];
      const tagObject = new CSS2DObject(tag); tagObject.position.set(-SLAB_W / 2, 0, SLAB_D / 2 + .5); tagObject.center.set(0, .5); group.add(tagObject);
      this.slabs.push({ tier, group, fill, edge, tag });
    }
    for (const node of nodes) {
      const place = places.get(node.id)!, slab = this.slabs.find(s => s.tier === node.tier)!;
      const body = new THREE.MeshStandardMaterial({ color: this.palette.panel.clone().lerp(this.palette[node.tier], .1), emissive: this.palette[node.tier], emissiveIntensity: 0, roughness: .45, metalness: .25, transparent: true });
      const mesh = new THREE.Mesh(new RoundedBoxGeometry(place.w, TILE_H, place.d, 3, .06), body);
      mesh.position.set(place.x, TILE_H / 2 + .02, place.z);
      mesh.userData.node = node;
      const edge = new THREE.LineBasicMaterial({ color: this.palette[node.tier], transparent: true, opacity: .3 });
      mesh.add(new THREE.LineSegments(new THREE.EdgesGeometry(new THREE.BoxGeometry(place.w, TILE_H, place.d)), edge));
      slab.group.add(mesh);
      const label = document.createElement('div'); label.className = 'scene-label'; label.textContent = node.title;
      if (node.note) { const note = document.createElement('small'); note.textContent = node.note; label.append(note); }
      const labelObject = new CSS2DObject(label); labelObject.position.y = TILE_H / 2 + .08; mesh.add(labelObject);
      // Blocks assemble tier by tier, left to right.
      const delay = .35 + tiers.indexOf(node.tier) * .22 + (place.x + 9) / 18 * .3;
      this.blocks.push({ node, mesh, body, edge, label, glow: 0, alpha: 1, line: .3, delay });
    }

    // A scan sheet that sweeps up through the stack in the hero and on every chapter change.
    this.sweepPlane = new THREE.Mesh(new THREE.PlaneGeometry(SLAB_W * 1.15, SLAB_D * 1.25), new THREE.ShaderMaterial({
      uniforms: { uColor: { value: this.palette.foundation }, uAlpha: { value: 0 } },
      vertexShader: 'varying vec2 vUv; void main() { vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }',
      fragmentShader: 'uniform vec3 uColor; uniform float uAlpha; varying vec2 vUv; void main() { vec2 d = abs(vUv - 0.5) * 2.0; float edge = smoothstep(1.0, 0.5, max(d.x, d.y)); float lines = 0.55 + 0.45 * step(0.5, fract(vUv.y * 48.0)); gl_FragColor = vec4(uColor, edge * lines * uAlpha); }',
      transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, side: THREE.DoubleSide,
    }));
    this.sweepPlane.rotation.x = -Math.PI / 2;
    this.scene.add(this.sweepPlane);

    this.composer = new EffectComposer(this.renderer);
    this.composer.addPass(new RenderPass(this.scene, this.camera));
    this.bloom = new UnrealBloomPass(new THREE.Vector2(256, 256), .62, .5, .16);
    this.composer.addPass(this.bloom);

    this.applyTheme();
    // Follow the system theme and the header toggle (data-theme) live.
    this.scheme.addEventListener('change', this.applyTheme);
    this.themeWatch.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

    const canvas = this.renderer.domElement;
    canvas.addEventListener('pointermove', this.onMove);
    canvas.addEventListener('pointerleave', this.onLeave);
    canvas.addEventListener('pointerdown', this.onDown);
    canvas.addEventListener('pointerup', this.onUp);
    this.resizer = new ResizeObserver(() => this.resize());
    this.resizer.observe(host);
    this.resize();
  }

  /** 0 is the hero orbit; 1 and above is the story view. Values between blend the two. */
  setProgress(progress: number) { this.progress = progress; }
  setView(view: StackView) {
    if (JSON.stringify(view.flows) !== JSON.stringify(this.view.flows)) { this.buildTrails(view.flows); if (this.intro >= 1 && !this.reduce) this.sweep = 0; }
    this.view = view;
  }
  setSelected(id: string | null) { this.selected = id; }
  setRunning(running: boolean) {
    if (running === this.running) return;
    this.running = running;
    if (running) { this.clock.getDelta(); this.loop(); } else cancelAnimationFrame(this.frame);
  }
  /** Re-read CSS tokens and restyle the scene: glow and additive light on dark, tinted solids on light. */
  applyTheme = () => {
    this.readPalette();
    const p = this.palette, light = this.light = p.bg.r + p.bg.g + p.bg.b > 1.5;
    (this.scene.fog as THREE.FogExp2).color.copy(p.bg);
    this.hemi.intensity = light ? 2.3 : .9; this.key.intensity = light ? 1.2 : 1.1;
    this.hemi.color.set(light ? 0xffffff : 0xb8c8ff); this.hemi.groundColor.set(light ? 0xdfe3ea : 0x101418);
    if (this.grid) { this.scene.remove(this.grid); this.grid.geometry.dispose(); (this.grid.material as THREE.Material).dispose(); }
    this.grid = new THREE.GridHelper(140, 70, p.line, p.line);
    Object.assign(this.grid.material, { transparent: true, opacity: light ? .8 : .35 });
    this.grid.position.y = -2.4; this.scene.add(this.grid);
    Object.assign(this.dust.material, { opacity: light ? .3 : .45 });
    (this.dust.material as THREE.PointsMaterial).color.copy(p.muted);
    for (const s of this.slabs) { s.fill.color.copy(p[s.tier]); s.edge.color.copy(p[s.tier]); }
    for (const b of this.blocks) { b.body.color.copy(p.panel).lerp(p[b.node.tier], .1); b.body.emissive.copy(p[b.node.tier]); b.edge.color.copy(p[b.node.tier]); }
    this.sweepPlane.material.uniforms.uColor.value = p.foundation;
    this.sweepPlane.material.blending = light ? THREE.NormalBlending : THREE.AdditiveBlending;
    this.sweepPlane.material.needsUpdate = true;
    this.bloom.enabled = !light;
    this.buildTrails(this.view.flows);
  };
  /** Re-read CSS tokens. */
  readPalette() {
    const read = (token: string) => { this.probe.style.color = `var(${token})`; return new THREE.Color(getComputedStyle(this.probe).color); };
    this.palette = { bg: read('--bg'), panel: read('--panel'), line: read('--line'), muted: read('--muted'), ink: read('--ink'), data: read('--flow-data'), control: read('--flow-control'), evidence: read('--flow-evidence') };
    for (const tier of tiers) this.palette[tier] = read(tierTokens[tier]);
    this.renderer.setClearColor(this.palette.bg);
  }
  dispose() {
    this.setRunning(false);
    this.resizer.disconnect();
    this.scheme.removeEventListener('change', this.applyTheme);
    this.themeWatch.disconnect();
    const canvas = this.renderer.domElement;
    canvas.removeEventListener('pointermove', this.onMove); canvas.removeEventListener('pointerleave', this.onLeave);
    canvas.removeEventListener('pointerdown', this.onDown); canvas.removeEventListener('pointerup', this.onUp);
    this.scene.traverse(o => {
      if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments || o instanceof THREE.Line || o instanceof THREE.Points) { o.geometry.dispose(); (o.material as THREE.Material).dispose(); }
    });
    this.composer.dispose();
    this.renderer.dispose();
    this.host.replaceChildren();
  }

  private resize() {
    const w = this.host.clientWidth, h = this.host.clientHeight;
    if (!w || !h) return;
    this.renderer.setSize(w, h); this.labels.setSize(w, h); this.composer.setSize(w, h);
    this.bloom.resolution.set(w / 2, h / 2);
    this.camera.aspect = w / h;
    const scale = h * this.renderer.getPixelRatio() / (2 * Math.tan(THREE.MathUtils.degToRad(this.camera.fov / 2)));
    for (const t of this.trails) (t.comets.material as THREE.ShaderMaterial).uniforms.uScale.value = scale;
  }

  private anchor(id: string, other: string, out: THREE.Vector3) {
    const block = this.blocks.find(b => b.node.id === id);
    if (block) return block.mesh.getWorldPosition(out);
    // `applications` is the whole tier: meet it straight above the other endpoint.
    this.blocks.find(b => b.node.id === other)!.mesh.getWorldPosition(out);
    return out.set(out.x, this.slabs[3].group.position.y + .05, out.z * .3);
  }
  private buildTrails(flows: Flow[]) {
    for (const t of this.trails) { this.scene.remove(t.line, t.comets); t.line.geometry.dispose(); (t.line.material as THREE.Material).dispose(); t.comets.geometry.dispose(); (t.comets.material as THREE.Material).dispose(); }
    const scale = this.host.clientHeight * this.renderer.getPixelRatio() / (2 * Math.tan(THREE.MathUtils.degToRad(this.camera.fov / 2)));
    this.trails = flows.map(flow => {
      const color = this.palette[flow.kind];
      const line = new THREE.Line(new THREE.BufferGeometry().setFromPoints(new Array(STEPS + 1).fill(0).map(() => new THREE.Vector3())),
        new THREE.LineBasicMaterial({ color, transparent: true, opacity: 0, depthTest: false, blending: this.light ? THREE.NormalBlending : THREE.AdditiveBlending }));
      const count = TRAIL * COMETS, size = new Float32Array(count), alpha = new Float32Array(count);
      for (let i = 0; i < count; i++) { const k = i % TRAIL; size[i] = .28 * (1 - k / TRAIL) + .04; alpha[i] = Math.pow(1 - k / TRAIL, 1.6); }
      const geometry = new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(new Float32Array(count * 3), 3))
        .setAttribute('aSize', new THREE.BufferAttribute(size, 1)).setAttribute('aAlpha', new THREE.BufferAttribute(alpha, 1));
      const comets = new THREE.Points(geometry, new THREE.ShaderMaterial({
        uniforms: { uColor: { value: color }, uFade: { value: 0 }, uScale: { value: scale }, uLight: { value: this.light ? 1 : 0 } },
        vertexShader: cometVertex, fragmentShader: cometFragment, transparent: true, depthWrite: false, depthTest: false, blending: this.light ? THREE.NormalBlending : THREE.AdditiveBlending,
      }));
      line.renderOrder = 5; comets.renderOrder = 6; line.frustumCulled = comets.frustumCulled = false;
      this.scene.add(line, comets);
      return { flow, line, comets, curve: new THREE.CubicBezierCurve3(new THREE.Vector3(), new THREE.Vector3(), new THREE.Vector3(), new THREE.Vector3()), phase: Math.random() };
    });
    this.fade = 0;
  }
  private updateCurve(t: Trail) {
    const a = new THREE.Vector3(), b = new THREE.Vector3(), c = t.curve, shift = pairOffset(t.flow, this.view.flows) * .38;
    this.anchor(t.flow.from, t.flow.to, a); this.anchor(t.flow.to, t.flow.from, b); a.x += shift; b.x += shift;
    c.v0.copy(a); c.v3.copy(b);
    if (Math.abs(a.y - b.y) < .5) {
      const lift = .8 + a.distanceTo(b) * .12;
      c.v0.y += TILE_H / 2; c.v3.y += TILE_H / 2;
      c.v1.copy(c.v0).setY(c.v0.y + lift); c.v2.copy(c.v3).setY(c.v3.y + lift);
    } else {
      const mid = (a.y + b.y) / 2;
      c.v1.set(a.x, mid, a.z); c.v2.set(b.x, mid, b.z);
    }
  }

  private pick(e: PointerEvent): StackNode | null {
    const r = this.renderer.domElement.getBoundingClientRect();
    this.raycaster.setFromCamera(new THREE.Vector2((e.clientX - r.left) / r.width * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1), this.camera);
    const hit = this.raycaster.intersectObjects(this.blocks.map(b => b.mesh), false).find(h => (h.object.userData.node as StackNode).select);
    return hit ? hit.object.userData.node as StackNode : null;
  }
  private onMove = (e: PointerEvent) => {
    const r = this.renderer.domElement.getBoundingClientRect();
    this.pointer.set((e.clientX - r.left) / r.width * 2 - 1, (e.clientY - r.top) / r.height * 2 - 1);
    const node = this.pick(e);
    this.hovered = node;
    this.renderer.domElement.style.cursor = node ? 'pointer' : 'default';
    this.events.hover(node, e.clientX, e.clientY);
  };
  private onLeave = () => { this.hovered = null; this.pointer.set(0, 0); this.events.hover(null, 0, 0); };
  private onDown = (e: PointerEvent) => { this.down = [e.clientX, e.clientY]; };
  private onUp = (e: PointerEvent) => {
    if (!this.down || Math.hypot(e.clientX - this.down[0], e.clientY - this.down[1]) > 6) return;
    this.events.select(this.pick(e));
  };

  private loop = () => {
    this.frame = requestAnimationFrame(this.loop);
    const dt = Math.min(this.clock.getDelta(), .1), time = this.clock.elapsedTime;
    const story = THREE.MathUtils.smoothstep(this.progress, 0, 1);
    const selectedBlock = this.blocks.find(b => b.node.id === this.selected);

    // Camera: slow hero orbit that settles into the isometric story view, then leans toward a selection.
    this.parallax.lerp(this.pointer, 1 - Math.exp(-3 * dt));
    if (!this.reduce) this.orbit += dt * (1 - story);
    // The hero sways within a flattering range instead of spinning to the stack's narrow side.
    const azimuth = THREE.MathUtils.lerp(.62 + Math.sin(this.orbit * .09) * .38, .72, story) + this.parallax.x * .08;
    const goalPolar = THREE.MathUtils.lerp(1.08, 1.0, story) + this.parallax.y * .04;
    const goalRadius = selectedBlock ? 34 : THREE.MathUtils.lerp(43, 47, story);
    const goal = new THREE.Vector3(0, THREE.MathUtils.lerp(2.2, 4.6, story), 0);
    if (selectedBlock) goal.lerp(selectedBlock.mesh.getWorldPosition(new THREE.Vector3()), .55);
    this.target.lerp(goal, 1 - Math.exp(-3 * dt));
    // Portrait screens need a wider shot to fit the stack's width.
    this.radius = damp(this.radius, goalRadius * Math.max(1, .6 / this.camera.aspect), 2.5, dt);
    this.polar = damp(this.polar, goalPolar, 3, dt);
    this.camera.position.setFromSphericalCoords(this.radius, this.polar, azimuth).add(this.target);
    this.camera.lookAt(this.target);
    // Desktop leaves the left side to the story copy (more in the hero); phones keep copy below the stack.
    const w = this.host.clientWidth, h = this.host.clientHeight;
    if (w > 900) this.camera.setViewOffset(w, h, -w * THREE.MathUtils.lerp(.21, .165, story), 0, w, h);
    else this.camera.setViewOffset(w, h, 0, h * .14, w, h);

    this.labels.domElement.classList.toggle('hero', story < .5);
    this.gap = damp(this.gap, THREE.MathUtils.lerp(HERO_GAP, STORY_GAP, story), 4, dt);
    // Intro: slabs rise into place, then blocks pop in; the first sweep follows.
    const before = this.intro;
    this.intro = this.reduce ? 1 : Math.min(1, this.intro + dt / 2.4);
    const appear = (delay: number) => this.reduce ? 1 : THREE.MathUtils.smoothstep(this.intro * 2.4 - delay, 0, .7);
    if (before < 1 && this.intro >= 1 && !this.reduce) this.sweep = 0;
    this.sweepTimer += dt;
    if (!this.reduce && story < .5 && this.intro >= 1 && this.sweepTimer > 7) { this.sweep = 0; this.sweepTimer = 0; }
    if (this.sweep < 1) this.sweep = Math.min(1, this.sweep + dt / 1.6);
    const sweepY = THREE.MathUtils.lerp(-1.5, 3 * this.gap + 1.5, THREE.MathUtils.smootherstep(this.sweep, 0, 1));
    const sweepAlpha = this.sweep < 1 ? Math.sin(Math.PI * this.sweep) * .5 : 0;
    this.sweepPlane.position.y = sweepY;
    this.sweepPlane.material.uniforms.uAlpha.value = sweepAlpha;

    this.slabs.forEach((s, i) => {
      const a = appear(i * .18);
      s.group.position.y = i * this.gap - (1 - a) * 3 + (this.reduce ? 0 : Math.sin(time * .7 + i) * .05 * (1 - story));
      const near = Math.max(0, 1 - Math.abs(s.group.position.y - sweepY) / 1.2) * sweepAlpha;
      s.edge.opacity = (THREE.MathUtils.lerp(.7, .4, story) + near) * a;
      s.fill.opacity = ((this.light ? .07 : .05) + near * .12) * a;
      s.tag.style.opacity = String(a);
    });

    for (const b of this.blocks) {
      const state = (this.view.states[b.node.id] as NodeState | undefined) ?? 'rest';
      const hover = this.hovered?.id === b.node.id ? .35 : 0;
      const glow = { rest: .22, on: .36, selected: .55, dim: .02 }[state] + hover * .6;
      b.glow = damp(b.glow, glow, 6, dt);
      const a = appear(b.delay);
      const scan = Math.max(0, 1 - Math.abs(b.mesh.parent!.position.y + b.mesh.position.y - sweepY) / 1.2) * sweepAlpha * 1.6;
      if (this.light) {
        // Light: tint the block toward its tier instead of glowing.
        b.body.emissiveIntensity = 0;
        b.body.color.copy(this.palette.panel).lerp(this.palette[b.node.tier], Math.min(1, b.glow * 1.5 + scan * .3));
      } else b.body.emissiveIntensity = b.glow * (state === 'rest' ? .6 + .4 * Math.sin(time * 1.3 + b.mesh.position.x) * (1 - story) : 1) + scan;
      b.alpha = damp(b.alpha, state === 'dim' ? .32 : 1, 6, dt);
      b.line = damp(b.line, { rest: .35, on: .95, selected: 1, dim: .1 }[state], 6, dt);
      b.body.opacity = b.alpha * a;
      b.body.depthWrite = b.body.opacity > .9;
      b.edge.opacity = Math.min(1, b.line + scan) * a;
      b.mesh.scale.setScalar(.6 + .4 * a);
      b.mesh.position.y = damp(b.mesh.position.y, TILE_H / 2 + .02 + (state === 'selected' ? .35 : 0), 6, dt);
      b.label.dataset.state = state;
    }

    const dust = this.dust.geometry.attributes.position as THREE.BufferAttribute;
    if (!this.reduce) for (let i = 0; i < dust.count; i++) { const y = dust.getY(i) + dt * .25; dust.setY(i, y > 30 ? -4 : y); }
    dust.needsUpdate = true;

    // Flows start once the stack has assembled.
    this.fade = this.intro < .8 ? 0 : Math.min(1, this.fade + dt * 1.6);
    for (const t of this.trails) {
      this.updateCurve(t);
      const line = t.line.geometry.attributes.position as THREE.BufferAttribute;
      for (let i = 0; i <= STEPS; i++) { const p = t.curve.getPoint(i / STEPS); line.setXYZ(i, p.x, p.y, p.z); }
      line.needsUpdate = true;
      (t.line.material as THREE.LineBasicMaterial).opacity = .28 * this.fade;
      (t.comets.material as THREE.ShaderMaterial).uniforms.uFade.value = this.fade;
      t.phase += dt * (this.reduce ? .08 : .32);
      const comets = t.comets.geometry.attributes.position as THREE.BufferAttribute;
      for (let c = 0; c < COMETS; c++) for (let k = 0; k < TRAIL; k++) {
        const p = t.curve.getPoint(((t.phase + c / COMETS - k * .012) % 1 + 1) % 1);
        comets.setXYZ(c * TRAIL + k, p.x, p.y, p.z);
      }
      comets.needsUpdate = true;
    }

    this.composer.render(dt);
    this.labels.render(this.scene, this.camera);
  };
}
