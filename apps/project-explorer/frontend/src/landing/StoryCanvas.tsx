import { useEffect, useRef, useState } from 'react';
import { useMotionValueEvent, type MotionValue } from 'motion/react';
import type { Integration } from '../overview-model';
import type { StackNode, StackView } from '../stack';
import { StoryScene } from './StoryScene';

/** Mount the story scene, drive it from scroll progress, and show a tooltip for the hovered block. */
export default function StoryCanvas({ nodes, integrations, view, selected, progress, describe, onSelect, onUnavailable }: {
  nodes: StackNode[]; integrations: Integration[]; view: StackView; selected: string | null; progress: MotionValue<number>;
  describe: (node: StackNode) => string; onSelect: (node: StackNode | null) => void; onUnavailable: () => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const scene = useRef<StoryScene | null>(null);
  const tip = useRef<HTMLDivElement>(null);
  const [hovered, setHovered] = useState<StackNode | null>(null);
  // Callbacks change with the selection; the scene reads the latest through this ref.
  const select = useRef(onSelect);
  select.current = onSelect;
  const latest = useRef({ view, selected });
  latest.current = { view, selected };

  useEffect(() => {
    let instance: StoryScene;
    try {
      instance = new StoryScene(host.current!, nodes, integrations, {
        hover(node, x, y) {
          setHovered(node);
          if (tip.current && node) tip.current.style.transform = `translate(${x + 16}px, ${y + 18}px)`;
        },
        select: node => select.current(node),
      });
    } catch { onUnavailable(); return; }
    scene.current = instance;
    instance.setProgress(progress.get());
    instance.setView(latest.current.view);
    instance.setSelected(latest.current.selected);
    // Render only while the story is on screen.
    const visible = new IntersectionObserver(([entry]) => instance.setRunning(entry.isIntersecting));
    visible.observe(host.current!);
    return () => { visible.disconnect(); instance.dispose(); scene.current = null; };
  }, [nodes, integrations, onUnavailable, progress]);

  useMotionValueEvent(progress, 'change', p => scene.current?.setProgress(p));
  useEffect(() => { scene.current?.setView(view); }, [view]);
  useEffect(() => { scene.current?.setSelected(selected); }, [selected]);

  return <>
    <div ref={host} className="story-canvas" />
    <div ref={tip} className={`scene-tip${hovered ? ' visible' : ''}`} aria-hidden="true">
      {hovered && <><strong>{hovered.title}</strong><span>{describe(hovered)}</span></>}
    </div>
  </>;
}
