import { useId, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from 'react';

const TYPEAHEAD_TIMEOUT_MS = 700;
const MENU_GAP = 6;
const MENU_MAX_HEIGHT = 280;
const MENU_MIN_WIDTH = 160;
const PAGE_OPTIONS = 10;

type Props<Value> = {
  id?: string; className?: string; 'aria-label'?: string; disabled?: boolean;
  value: Value; options: readonly { value: Value; label: string }[]; onChange: (value: Value) => void;
};

/** Themed single-choice list. Keyboard browsing commits on Enter, Space or Tab; Escape cancels. */
export function Select<Value extends string | number>({ id, className = '', 'aria-label': label, disabled = false, value, options, onChange }: Props<Value>) {
  const uniqueId = useId();
  const controlId = id ?? `select-${uniqueId}`, listId = `${controlId}-options`;
  const root = useRef<HTMLDivElement>(null), trigger = useRef<HTMLButtonElement>(null), list = useRef<HTMLDivElement>(null);
  const search = useRef({ text: '', time: 0 });
  const [expanded, setExpanded] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const [position, setPosition] = useState<CSSProperties>({ visibility: 'hidden' });
  const selected = options.findIndex(option => option.value === value);
  const active = Math.min(activeIndex, options.length - 1);
  const unavailable = disabled || options.length === 0;
  const open = expanded && !unavailable;
  const close = () => { setExpanded(false); search.current.text = ''; };
  const show = (index = Math.max(0, selected)) => { setActiveIndex(index); setExpanded(true); search.current.text = ''; };
  const choose = (index: number) => {
    close();
    if (!unavailable && options[index] && options[index].value !== value) onChange(options[index].value);
  };

  useLayoutEffect(() => { if (unavailable) setExpanded(false); }, [unavailable]);
  useLayoutEffect(() => {
    if (!open || !trigger.current || !list.current) return;
    // Keep the menu inside the viewport, including when the control is near its bottom edge.
    const bounds = trigger.current.getBoundingClientRect();
    const viewport = document.documentElement;
    const below = viewport.clientHeight - bounds.bottom - MENU_GAP * 2;
    const above = bounds.top - MENU_GAP * 2;
    const naturalHeight = list.current.scrollHeight + list.current.offsetHeight - list.current.clientHeight;
    const upwards = below < Math.min(naturalHeight, MENU_MAX_HEIGHT) && above > below;
    const maxHeight = Math.max(0, Math.min(MENU_MAX_HEIGHT, upwards ? above : below));
    const width = Math.min(Math.max(bounds.width, MENU_MIN_WIDTH), viewport.clientWidth - MENU_GAP * 2);
    setPosition({ width, maxHeight, left: Math.max(MENU_GAP, Math.min(bounds.left, viewport.clientWidth - width - MENU_GAP)), top: upwards ? bounds.top - MENU_GAP - Math.min(naturalHeight, maxHeight) : bounds.bottom + MENU_GAP });
    const outside = (event: PointerEvent) => { if (!root.current?.contains(event.target as Node)) setExpanded(false); };
    const scroll = (event: Event) => { if (!list.current?.contains(event.target as Node)) setExpanded(false); };
    const resize = () => setExpanded(false);
    document.addEventListener('pointerdown', outside);
    window.addEventListener('scroll', scroll, true);
    window.addEventListener('resize', resize);
    return () => {
      document.removeEventListener('pointerdown', outside);
      window.removeEventListener('scroll', scroll, true);
      window.removeEventListener('resize', resize);
    };
  }, [open, options.length]);
  useLayoutEffect(() => {
    if (open) list.current?.children[active]?.scrollIntoView({ block: 'nearest' });
  }, [open, active, position]);

  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (unavailable || event.ctrlKey || event.metaKey) return;
    const key = event.key;
    if (key === 'Tab') { if (open) choose(active); return; }
    if (key === 'Escape') { if (open) { event.preventDefault(); event.stopPropagation(); close(); } return; }
    if (event.altKey && key !== 'ArrowDown' && key !== 'ArrowUp') return;
    if (event.altKey && key === 'ArrowUp') { if (open) { event.preventDefault(); choose(active); } return; }
    if (key === 'Enter' || (key === ' ' && (!open || !search.current.text || Date.now() - search.current.time > TYPEAHEAD_TIMEOUT_MS))) {
      event.preventDefault();
      if (open) choose(active); else show();
    } else if (['ArrowDown', 'ArrowUp', 'Home', 'End', 'PageDown', 'PageUp'].includes(key)) {
      event.preventDefault();
      search.current.text = '';
      const offset = key === 'PageDown' ? PAGE_OPTIONS : key === 'PageUp' ? -PAGE_OPTIONS : key === 'ArrowDown' ? 1 : -1;
      const next = key === 'Home' ? 0 : key === 'End' ? options.length - 1 : !open ? Math.max(0, selected) : Math.max(0, Math.min(options.length - 1, active + offset));
      show(next);
    } else if (key.length === 1 && !event.altKey) {
      event.preventDefault();
      const now = Date.now();
      const text = (now - search.current.time <= TYPEAHEAD_TIMEOUT_MS ? search.current.text : '') + key.toLocaleLowerCase();
      const repeated = [...text].every(character => character === text[0]);
      const prefix = repeated ? text[0] : text;
      const start = open ? active : Math.max(0, selected);
      const indexes = options.map((_, offset) => (start + offset + (repeated ? 1 : 0)) % options.length);
      const match = indexes.find(index => options[index].label.toLocaleLowerCase().startsWith(prefix));
      if (!open) show();
      if (match !== undefined) setActiveIndex(match);
      search.current = { text, time: now };
    }
  };

  return <div ref={root} className={`select ${className}`} onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) close(); }}>
    <button ref={trigger} id={controlId} type="button" className="select-trigger" role="combobox" aria-label={label} aria-haspopup="listbox" aria-expanded={open} aria-controls={open ? listId : undefined} aria-activedescendant={open ? `${listId}-${active}` : undefined} disabled={unavailable} onKeyDown={onKeyDown} onClick={() => { if (open) close(); else show(); }}>
      <span>{options[selected]?.label ?? 'Choose…'}</span><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 6 4 4 4-4"/></svg>
    </button>
    {open && <div ref={list} id={listId} className="select-menu" role="listbox" aria-label={label} aria-labelledby={label ? undefined : controlId} style={position}>
      {options.map((option, index) => <div key={option.value} id={`${listId}-${index}`} role="option" aria-selected={index === active} className="select-option" onPointerMove={event => { if (event.pointerType === 'mouse') setActiveIndex(index); }} onPointerDown={event => event.preventDefault()} onClick={() => { choose(index); trigger.current?.focus(); }}>
        <span>{option.label}</span><svg viewBox="0 0 16 16" aria-hidden="true" className={option.value === value ? 'select-check' : 'select-check hidden'}><path d="m3 8 3 3 7-7"/></svg>
      </div>)}
    </div>}
  </div>;
}
