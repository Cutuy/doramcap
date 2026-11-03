import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { CSS2DObject, CSS2DRenderer } from 'three/addons/renderers/CSS2DRenderer.js';
import { RoundedBoxGeometry } from 'three/addons/geometries/RoundedBoxGeometry.js';
import type { Integration } from './overview-model';
import { pairOffset, type StackNode, type StackView, type Tier } from './stack';

// Plain sRGB keeps scene colors identical to the CSS tokens.
THREE.ColorManagement.enabled = false;

const tiers: Tier[] = ['compute', 'platform', 'foundation', 'applications'];
const tierLabels: Record<Tier, string> = { applications: 'Your applications', foundation: 'doramcap', platform: 'Platform', compute: 'Compute' };
const tierColors: Record<Tier, string> = { applications: '--tier-apps', foundation: '--tier-core', platform: '--tier-platform', compute: '--tier-compute' };
const GAP = 3, SLAB_W = 17, SLAB_D = 7, TILE_H = .46;
interface Place { x: number; z: number; w: number; d: number; }

/** 3D coordinates per node; the foundation uses two rows so doramcap gets the most room. */
function layout(nodes: StackNode[], integrations: Integration[]): Map<string, Place> {
  const places = new Map<string, Place>();
  const row = (ids: string[], spacing: number, w: number, z = 0, d = 1.5) => ids.forEach((id, i) => places.set(id, { x: (i - (ids.length - 1) / 2) * spacing, z, w, d }));
  const of = (t: Tier) => nodes.filter(n => n.tier === t).map(n => n.id);
  row(of('applications'), 3.8, 3.1);
  const foundation = of('foundation');
  row(foundation.slice(0, 4), 4, 3.2, -1.5);
  row(foundation.slice(4), 4, 3.2, 1.5);
  const runtime = integrations.filter(i => i.group === 'execution').map(i => i.id);
  row([...runtime, ...integrations.filter(i => i.group !== 'execution').map(i => i.id)], 2.6, 2.2);
  row(of('compute'), 5, 4.2);
  return places;
}

function colorOf(probe: HTMLElement, token: string) {
  probe.style.color = `var(${token})`;
  return new THREE.Color(getComputedStyle(probe).color);
}

/** Interactive isometric stack. Clicking a selectable block navigates to its overview link. */
export default function Stack3D({ nodes, integrations, view, hrefFor, onUnavailable }: {
  nodes: StackNode[]; integrations: Integration[]; view: StackView;
  hrefFor: (node: StackNode) => string | undefined; onUnavailable: () => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const current = useRef(view);
  current.current = view;
  // Links change with the selection; read them at click time instead of rebuilding the scene.
  const links = useRef(hrefFor);
  links.current = hrefFor;

  useEffect(() => {
    const root = host.current!;
    let renderer: THREE.WebGLRenderer;
    try { renderer = new THREE.WebGLRenderer({ antialias: true }); } catch { onUnavailable(); return; }
    renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    root.appendChild(renderer.domElement);
    const labels = new CSS2DRenderer();
    labels.domElement.className = 'stack3d-labels';
    root.appendChild(labels.domElement);
    const probe = document.createElement('span');
    probe.hidden = true;
    root.appendChild(probe);

    const scene = new THREE.Scene();
    const camera = new THREE.OrthographicCamera(-10, 10, 10, -10, .1, 400);
    camera.position.set(30, 25, 34);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.target.set(0, 4.2, 0);
    Object.assign(controls, { enableDamping: true, dampingFactor: .08, enablePan: false, minZoom: .75, maxZoom: 2.2, minPolarAngle: .55, maxPolarAngle: 1.25 });
    const azimuth = Math.atan2(30, 34);
    controls.minAzimuthAngle = azimuth - .75; controls.maxAzimuthAngle = azimuth + .75;
    scene.add(new THREE.HemisphereLight(0xffffff, 0xc8c8c8, 1.9));
    const sun = new THREE.DirectionalLight(0xffffff, .85); sun.position.set(-6, 20, 14); scene.add(sun);

    const places = layout(nodes, integrations);
    const slabs = tiers.map((tier, i) => {
      const group = new THREE.Group(); group.position.y = i * GAP; scene.add(group);
      const fill = new THREE.MeshBasicMaterial({ transparent: true, opacity: .9, depthWrite: false });
      group.add(new THREE.Mesh(new THREE.BoxGeometry(SLAB_W, .1, SLAB_D), fill));
      const edge = new THREE.LineBasicMaterial({ transparent: true, opacity: .6 });
      group.add(new THREE.LineSegments(new THREE.EdgesGeometry(new THREE.BoxGeometry(SLAB_W, .1, SLAB_D)), edge));
      const label = document.createElement('div'); label.className = `stack3d-tier tier-${tier}`; label.textContent = tierLabels[tier];
      const tag = new CSS2DObject(label); tag.position.set(-SLAB_W / 2 + .2, .1, SLAB_D / 2 + .55); tag.center.set(0, .5); group.add(tag);
      return { tier, group, fill, edge };
    });
    const blocks = nodes.map(node => {
      const place = places.get(node.id)!, slab = slabs.find(s => s.tier === node.tier)!;
      const material = new THREE.MeshStandardMaterial({ roughness: .85, transparent: true });
      const mesh = new THREE.Mesh(new RoundedBoxGeometry(place.w, TILE_H, place.d, 3, .07), material);
      mesh.position.set(place.x, .05 + TILE_H / 2, place.z);
      mesh.userData.node = node;
      slab.group.add(mesh);
      const label = document.createElement('div'); label.className = 'stack3d-label';
      label.textContent = node.title;
      if (node.note) { const note = document.createElement('small'); note.textContent = node.note; label.append(note); }
      const tag = new CSS2DObject(label); tag.position.y = TILE_H / 2 + .05; mesh.add(tag);
      return { node, mesh, material, label };
    });

    const dot = new THREE.CanvasTexture((() => { const c = document.createElement('canvas'); c.width = c.height = 32; const g = c.getContext('2d')!; g.fillStyle = '#fff'; g.beginPath(); g.arc(16, 16, 13, 0, Math.PI * 2); g.fill(); return c; })());
    const PER = 6, STEPS = 48;
    let trails: { line: THREE.Line; points: THREE.Points; curve: THREE.CubicBezierCurve3; phase: number }[] = [];
    let drawn = '';
    const va = new THREE.Vector3(), vb = new THREE.Vector3();
    function anchor(id: string, other: string, out: THREE.Vector3) {
      const block = blocks.find(b => b.node.id === id);
      if (block) return block.mesh.getWorldPosition(out);
      // `applications` is the whole tier: meet it straight above the other endpoint.
      blocks.find(b => b.node.id === other)!.mesh.getWorldPosition(out);
      return out.set(out.x, 3 * GAP + .1, 0);
    }
    function rebuild(flows: StackView['flows'], palette: Record<string, THREE.Color>) {
      for (const t of trails) { scene.remove(t.line, t.points); t.line.geometry.dispose(); t.points.geometry.dispose(); (t.line.material as THREE.Material).dispose(); (t.points.material as THREE.Material).dispose(); }
      trails = flows.map(f => {
        const shift = pairOffset(f, flows) * .35;
        anchor(f.from, f.to, va); anchor(f.to, f.from, vb); va.x += shift; vb.x += shift;
        const curve = new THREE.CubicBezierCurve3(va.clone(), new THREE.Vector3(), new THREE.Vector3(), vb.clone());
        if (Math.abs(va.y - vb.y) < .5) {
          const lift = .7 + va.distanceTo(vb) * .12;
          curve.v0.y += TILE_H / 2; curve.v3.y += TILE_H / 2;
          curve.v1.copy(curve.v0); curve.v1.y += lift; curve.v2.copy(curve.v3); curve.v2.y += lift;
        } else {
          const my = (va.y + vb.y) / 2;
          curve.v1.set(va.x, my, va.z); curve.v2.set(vb.x, my, vb.z);
        }
        const color = palette[`flow-${f.kind}`];
        const line = new THREE.Line(new THREE.BufferGeometry().setFromPoints(curve.getPoints(STEPS)), new THREE.LineBasicMaterial({ color, transparent: true, opacity: .8, depthTest: false }));
        const points = new THREE.Points(new THREE.BufferGeometry().setFromPoints(Array.from({ length: PER }, () => new THREE.Vector3())),
          new THREE.PointsMaterial({ color, size: 7, sizeAttenuation: false, map: dot, transparent: true, alphaTest: .3, depthTest: false }));
        line.renderOrder = 5; points.renderOrder = 6;
        scene.add(line, points);
        return { line, points, curve, phase: Math.random() };
      });
    }

    let palette: Record<string, THREE.Color> = {};
    function readPalette() {
      palette = Object.fromEntries(['--panel', ...Object.values(tierColors), '--flow-data', '--flow-control', '--flow-evidence'].map(t => [t.slice(2), colorOf(probe, t)]));
      renderer.setClearColor(palette.panel);
      drawn = '';
    }
    readPalette();
    const scheme = matchMedia('(prefers-color-scheme: dark)');
    scheme.addEventListener('change', readPalette);

    function fit() {
      const w = root.clientWidth, h = root.clientHeight, a = w / h, half = Math.max(7.6, 12.4 / a);
      Object.assign(camera, { left: -half * a, right: half * a, top: half, bottom: -half });
      camera.updateProjectionMatrix();
      renderer.setSize(w, h); labels.setSize(w, h);
    }
    const resize = new ResizeObserver(fit); resize.observe(root);

    const ray = new THREE.Raycaster(), pointer = new THREE.Vector2();
    function pick(e: PointerEvent) {
      const r = renderer.domElement.getBoundingClientRect();
      pointer.set((e.clientX - r.left) / r.width * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
      ray.setFromCamera(pointer, camera);
      const node = ray.intersectObjects(blocks.map(b => b.mesh), false)[0]?.object.userData.node as StackNode | undefined;
      return node && links.current(node);
    }
    let down: [number, number] | null = null;
    const onDown = (e: PointerEvent) => { down = [e.clientX, e.clientY]; };
    const onUp = (e: PointerEvent) => { if (down && Math.hypot(e.clientX - down[0], e.clientY - down[1]) < 5) { const href = pick(e); if (href) window.location.hash = href.slice(1); } };
    const onMove = (e: PointerEvent) => { renderer.domElement.style.cursor = pick(e) ? 'pointer' : 'grab'; };
    renderer.domElement.addEventListener('pointerdown', onDown);
    renderer.domElement.addEventListener('pointerup', onUp);
    renderer.domElement.addEventListener('pointermove', onMove);

    const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
    const clock = new THREE.Clock(), tint = new THREE.Color();
    let frame = 0;
    function animate() {
      frame = requestAnimationFrame(animate);
      const dt = Math.min(clock.getDelta(), .1), time = clock.elapsedTime, k = 1 - Math.pow(.001, dt);
      const { states, flows } = current.current;
      const rise = reduce ? 1 : 1 - Math.pow(1 - Math.min(time / 1.4, 1), 3);
      slabs.forEach((s, i) => {
        s.group.position.y = i * GAP * rise;
        s.fill.color.copy(palette.panel).lerp(palette[tierColors[s.tier].slice(2)], .09);
        s.edge.color.copy(palette[tierColors[s.tier].slice(2)]);
      });
      for (const b of blocks) {
        const state = states[b.node.id] ?? 'rest';
        tint.copy(palette.panel).lerp(palette[tierColors[b.node.tier].slice(2)], state === 'dim' ? .12 : state === 'selected' ? 1 : state === 'on' ? .8 : .2);
        b.material.color.lerp(tint, k);
        b.material.opacity += ((state === 'dim' ? .3 : 1) - b.material.opacity) * k;
        b.material.depthWrite = b.material.opacity > .9;
        b.mesh.position.y += (.05 + TILE_H / 2 + (state === 'selected' ? .25 : 0) - b.mesh.position.y) * k;
        b.label.dataset.state = state;
      }
      const key = JSON.stringify(flows);
      if (rise === 1 && key !== drawn) { drawn = key; rebuild(flows, palette); }
      for (const t of trails) {
        t.phase += dt * (reduce ? .12 : .4);
        const p = t.points.geometry.attributes.position as THREE.BufferAttribute;
        for (let i = 0; i < PER; i++) { const v = t.curve.getPoint((t.phase + i / PER) % 1); p.setXYZ(i, v.x, v.y, v.z); }
        p.needsUpdate = true;
      }
      controls.update();
      renderer.render(scene, camera);
      labels.render(scene, camera);
    }
    animate();

    return () => {
      cancelAnimationFrame(frame);
      resize.disconnect();
      scheme.removeEventListener('change', readPalette);
      controls.dispose();
      rebuild([], palette);
      scene.traverse(o => { if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments) { o.geometry.dispose(); (o.material as THREE.Material).dispose(); } });
      dot.dispose();
      renderer.dispose();
      root.replaceChildren();
    };
  }, [nodes, integrations, onUnavailable]);

  return <div ref={host} className="stack3d" />;
}
