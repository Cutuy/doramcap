use crate::capture::{Core, FIELDS, Field, LOCAL, Record, SpanState, TEXT};
use std::{
    fmt::{self, Write},
    sync::{Arc, atomic::Ordering},
};
use tracing_core::{
    Event, Metadata, Subscriber,
    field::{Field as TraceField, Visit},
    span::{Attributes, Current, Id, Record as SpanRecord},
};

/// A bounded subscriber. Registry bookkeeping uses a short standard mutex;
/// capture never waits for a free buffer or performs output I/O.
pub struct ActivitySubscriber(pub(crate) Arc<Core>);
impl ActivitySubscriber {
    fn record_for(&self, id: u64, kind: u32) -> Option<Record> {
        self.0.context(id).map(|s| Record {
            kind,
            span: s.id,
            root: s.root,
            parent: s.parent,
            name: s.meta.name(),
            ..Default::default()
        })
    }
}
impl Subscriber for ActivitySubscriber {
    fn enabled(&self, meta: &Metadata<'_>) -> bool {
        meta.name().len() <= TEXT
            && meta.fields().iter().all(|f| f.name().len() <= TEXT)
            && (meta.target() == "activity" || meta.target().starts_with("activity::"))
            && self.0.accepting.load(Ordering::Acquire)
    }
    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let parent_state = if let Some(id) = attrs.parent() {
            self.0.context(id.into_u64()).map(|s| (s.root, s.id))
        } else if attrs.is_contextual() {
            self.0.contextual_parent()
        } else {
            None
        };
        let state = {
            let mut r = self.0.registry.lock().unwrap();
            let Some(index) = r.slots.iter().position(Option::is_none) else {
                self.0.losses.spans.fetch_add(1, Ordering::Relaxed);
                return Id::from_u64(u64::MAX);
            };
            let Some(id) = r
                .next
                .checked_mul(r.slots.len() as u64)
                .and_then(|n| n.checked_add(index as u64))
                .filter(|id| *id < u64::MAX)
            else {
                self.0.losses.spans.fetch_add(1, Ordering::Relaxed);
                return Id::from_u64(u64::MAX);
            };
            r.next += 1;
            let s = SpanState {
                id,
                refs: 1,
                root: parent_state.map_or(id, |s| s.0),
                parent: parent_state.map_or(0, |s| s.1),
                meta: attrs.metadata(),
            };
            r.slots[index] = Some(s);
            s
        };
        self.0.emit(
            Record {
                kind: 0,
                span: state.id,
                root: state.root,
                parent: state.parent,
                name: state.meta.name(),
                ..Default::default()
            },
            |r, b| {
                attrs.record(&mut Visitor {
                    r,
                    b,
                    core: &self.0,
                })
            },
        );
        Id::from_u64(state.id)
    }
    fn record(&self, id: &Id, values: &SpanRecord<'_>) {
        if let Some(r) = self.record_for(id.into_u64(), 2) {
            self.0.emit(r, |r, b| {
                values.record(&mut Visitor {
                    r,
                    b,
                    core: &self.0,
                })
            });
        }
    }
    fn record_follows_from(&self, id: &Id, follows: &Id) {
        if let (Some(mut r), Some(other)) = (
            self.record_for(id.into_u64(), 3),
            self.0.context(follows.into_u64()),
        ) {
            r.linked = other.id;
            r.linked_root = other.root;
            self.0.emit(r, |_, _| {});
        }
    }
    fn event(&self, event: &Event<'_>) {
        let id = event
            .parent()
            .map(Id::into_u64)
            .or_else(|| event.is_contextual().then(|| self.0.current()).flatten());
        let state = id.and_then(|id| self.0.context(id));
        let r = Record {
            kind: 1,
            span: state.map_or(0, |s| s.id),
            root: state.map_or(0, |s| s.root),
            name: event.metadata().name(),
            ..Default::default()
        };
        self.0.emit(r, |r, b| {
            event.record(&mut Visitor {
                r,
                b,
                core: &self.0,
            })
        });
    }
    fn enter(&self, id: &Id) {
        LOCAL.with(|l| {
            if let Some(s) = l
                .borrow_mut()
                .as_mut()
                .filter(|s| Arc::ptr_eq(&s.core, &self.0))
            {
                if s.overflow_depth == 0 && s.stack.len() < self.0.config.max_depth {
                    s.stack.push(id.into_u64());
                } else {
                    s.overflow_depth += 1;
                    self.0.losses.spans.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }
    fn exit(&self, id: &Id) {
        LOCAL.with(|l| {
            if let Some(s) = l
                .borrow_mut()
                .as_mut()
                .filter(|s| Arc::ptr_eq(&s.core, &self.0))
            {
                if s.overflow_depth > 0 {
                    s.overflow_depth -= 1;
                    return;
                }
                if let Some(i) = s.stack.iter().rposition(|v| *v == id.into_u64()) {
                    s.stack.remove(i);
                }
                if let Some(block) = s.block.take() {
                    s.core.publish(block);
                }
            }
        });
    }
    fn clone_span(&self, id: &Id) -> Id {
        let mut r = self.0.registry.lock().unwrap();
        let n = r.slots.len();
        if let Some(s) = r.slots[id.into_u64() as usize % n]
            .as_mut()
            .filter(|s| s.id == id.into_u64())
        {
            s.refs = s
                .refs
                .checked_add(1)
                .expect("span reference count overflow");
        }
        id.clone()
    }
    fn try_close(&self, id: Id) -> bool {
        let state = {
            let mut r = self.0.registry.lock().unwrap();
            let n = r.slots.len();
            let slot = &mut r.slots[id.into_u64() as usize % n];
            let Some(s) = slot.as_mut().filter(|s| s.id == id.into_u64()) else {
                return true;
            };
            s.refs -= 1;
            if s.refs > 0 {
                return false;
            }
            slot.take().unwrap()
        };
        self.0.emit(
            Record {
                kind: 4,
                span: state.id,
                root: state.root,
                parent: state.parent,
                name: state.meta.name(),
                ..Default::default()
            },
            |_, _| {},
        );
        true
    }
    fn current_span(&self) -> Current {
        self.0
            .current()
            .and_then(|id| self.0.context(id))
            .map_or_else(Current::none, |s| Current::new(Id::from_u64(s.id), s.meta))
    }
}
struct Visitor<'a> {
    r: &'a mut Record,
    b: &'a mut Vec<u8>,
    core: &'a Core,
}
impl Visitor<'_> {
    fn add(&mut self, field: Field) {
        if self.r.count == FIELDS {
            self.core.losses.fields.fetch_add(1, Ordering::Relaxed);
        } else {
            self.r.fields[self.r.count] = field;
            self.r.count += 1;
        }
    }
    fn text(&mut self, f: &TraceField, s: &str) {
        if self.r.count == FIELDS {
            self.core.losses.fields.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let available = self.core.config.string_bytes_per_block - self.b.len();
        let mut len = s.len().min(TEXT).min(available);
        while !s.is_char_boundary(len) {
            len -= 1;
        }
        if len < s.len() {
            self.core.losses.fields.fetch_add(1, Ordering::Relaxed);
        }
        let offset = self.b.len() as u32;
        self.b.extend_from_slice(&s.as_bytes()[..len]);
        self.add(Field {
            name: f.name(),
            kind: 4,
            offset,
            len: len as u32,
            ..Default::default()
        });
    }
}
impl Visit for Visitor<'_> {
    fn record_u64(&mut self, f: &TraceField, v: u64) {
        self.add(Field {
            name: f.name(),
            kind: 0,
            bits: v,
            ..Default::default()
        })
    }
    fn record_i64(&mut self, f: &TraceField, v: i64) {
        self.add(Field {
            name: f.name(),
            kind: 1,
            bits: v as u64,
            ..Default::default()
        })
    }
    fn record_f64(&mut self, f: &TraceField, v: f64) {
        self.add(Field {
            name: f.name(),
            kind: 2,
            bits: v.to_bits(),
            ..Default::default()
        })
    }
    fn record_bool(&mut self, f: &TraceField, v: bool) {
        self.add(Field {
            name: f.name(),
            kind: 3,
            bits: v as u64,
            ..Default::default()
        })
    }
    fn record_str(&mut self, f: &TraceField, v: &str) {
        self.text(f, v)
    }
    fn record_debug(&mut self, f: &TraceField, v: &dyn fmt::Debug) {
        let mut text = Text {
            bytes: [0; TEXT],
            len: 0,
        };
        if write!(&mut text, "{v:?}").is_err() {
            self.core.losses.fields.fetch_add(1, Ordering::Relaxed);
        }
        self.text(f, std::str::from_utf8(&text.bytes[..text.len]).unwrap());
    }
}
struct Text {
    bytes: [u8; TEXT],
    len: usize,
}
impl Write for Text {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut n = s.len().min(TEXT - self.len);
        while !s.is_char_boundary(n) {
            n -= 1;
        }
        self.bytes[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        if n < s.len() { Err(fmt::Error) } else { Ok(()) }
    }
}
