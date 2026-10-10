//! Typed events ordered by simulation time and then insertion order.
use crate::{Error, Result, error::nonnegative};
use alloc::{collections::BinaryHeap, vec::Vec};
use core::cmp::Ordering;

/// Stable queue-local event identifier used for cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(u64);
impl EventId {
    /// Integer identifier for logging or snapshots.
    pub fn get(self) -> u64 {
        self.0
    }
}
/// A typed payload and its scheduled time.
#[derive(Debug, Clone, PartialEq)]
pub struct Scheduled<E> {
    /// Queue-local event identity.
    pub id: EventId,
    /// Absolute simulation time.
    pub at: f64,
    /// Application-defined event payload.
    pub event: E,
}
#[derive(Debug, Clone)]
struct Entry<E>(Scheduled<E>);
impl<E> PartialEq for Entry<E> {
    fn eq(&self, other: &Self) -> bool {
        self.0.at == other.0.at && self.0.id == other.0.id
    }
}
impl<E> Eq for Entry<E> {}
impl<E> PartialOrd for Entry<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<E> Ord for Entry<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .at
            .total_cmp(&self.0.at)
            .then_with(|| other.0.id.cmp(&self.0.id))
    }
}
/// Event queue driven explicitly by simulation time. No threads or wall clock.
#[derive(Debug, Clone)]
pub struct EventQueue<E> {
    heap: BinaryHeap<Entry<E>>,
    now: f64,
    next_id: u64,
}
impl<E> Default for EventQueue<E> {
    fn default() -> Self {
        Self::new()
    }
}
impl<E> EventQueue<E> {
    /// Creates an empty queue at time zero.
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            now: 0.0,
            next_id: 0,
        }
    }
    /// Current queue time.
    pub fn now(&self) -> f64 {
        self.now
    }
    /// Number of pending events.
    pub fn len(&self) -> usize {
        self.heap.len()
    }
    /// Whether there are no pending events.
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
    /// Schedules at an absolute finite time at or after `now`.
    pub fn schedule_at(&mut self, at: f64, event: E) -> Result<EventId> {
        nonnegative(at)?;
        if at < self.now {
            return Err(Error::PastEvent);
        }
        let next = self.next_id.checked_add(1).ok_or(Error::Overflow)?;
        let id = EventId(self.next_id);
        // Canonicalize negative zero so equality and ordering agree.
        self.heap.push(Entry(Scheduled {
            id,
            at: if at == 0.0 { 0.0 } else { at },
            event,
        }));
        self.next_id = next;
        Ok(id)
    }
    /// Schedules relative to the queue's current time.
    pub fn schedule_after(&mut self, delay: f64, event: E) -> Result<EventId> {
        nonnegative(delay)?;
        self.schedule_at(self.now + delay, event)
    }
    /// Inspects the next event without removing it.
    pub fn peek(&self) -> Option<&Scheduled<E>> {
        self.heap.peek().map(|entry| &entry.0)
    }
    /// Removes a pending event; cancellation is O(number of pending events).
    pub fn cancel(&mut self, id: EventId) -> bool {
        let old = self.len();
        self.heap.retain(|entry| entry.0.id != id);
        self.len() != old
    }
    /// Pops the next event and advances queue time to that event.
    /// Useful when handlers must schedule follow-up events relative to dispatch time.
    pub fn pop_next(&mut self) -> Option<Scheduled<E>> {
        let entry = self.heap.pop()?.0;
        self.now = entry.at;
        Some(entry)
    }
    /// Advances time and returns due events in stable order (including the endpoint).
    /// Follow-up events scheduled after this call are relative to `until`, not their old due times.
    pub fn advance_to(&mut self, until: f64) -> Result<Vec<Scheduled<E>>> {
        nonnegative(until)?;
        if until < self.now {
            return Err(Error::PastEvent);
        }
        let mut due = Vec::new();
        while self.heap.peek().is_some_and(|entry| entry.0.at <= until) {
            due.push(self.heap.pop().expect("queue has an event").0);
        }
        self.now = until;
        Ok(due)
    }
    /// Clears pending events but preserves current time and identifier sequence.
    pub fn clear(&mut self) {
        self.heap.clear();
    }
}
