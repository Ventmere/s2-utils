use std::sync::{Arc, RwLock, Weak};

type Container<A, R> = RwLock<Vec<Slot<A, R>>>;

enum Slot<A, R> {
  Empty,
  Occupied(i64, Box<dyn Fn(A) -> R + Send>),
}

pub struct SlotHandle<A, R> {
  pos: usize,
  container_ref: Weak<Container<A, R>>,
}

pub struct Delegate<A, R> {
  slots: Arc<Container<A, R>>,
  max: RwLock<i64>,
}

impl<A, R> Delegate<A, R> {
  pub fn new() -> Self {
    Delegate {
      slots: Arc::new(RwLock::new(vec![])),
      max: RwLock::new(0),
    }
  }

  pub fn len(&self) -> usize {
    self
      .slots
      .read()
      .unwrap()
      .iter()
      .filter(|slot| match **slot {
        Slot::Occupied(_, _) => true,
        _ => false,
      })
      .count()
  }

  fn make_occupied(&self, b: Box<dyn Fn(A) -> R + Send>) -> Slot<A, R> {
    let mut max = self.max.write().unwrap();
    let next = (*max) + 1;
    *max = next;
    Slot::Occupied(next, b)
  }

  pub fn add<F>(&self, cb: F) -> SlotHandle<A, R>
  where
    F: Fn(A) -> R + 'static + Send,
  {
    let mut slots = self.slots.write().unwrap();

    let empty_pos = slots.iter().position(|slot| match *slot {
      Slot::Empty => true,
      _ => false,
    });

    let pos = if let Some(pos) = empty_pos {
      ::std::mem::replace(
        slots.get_mut(pos).unwrap(),
        self.make_occupied(Box::new(cb)),
      );
      pos
    } else {
      slots.push(self.make_occupied(Box::new(cb)));
      slots.len() - 1
    };

    SlotHandle {
      pos,
      container_ref: Arc::downgrade(&self.slots),
    }
  }

  pub fn remove(&self, handle: SlotHandle<A, R>) {
    let fail = {
      let mut slots = self.slots.write().unwrap();
      if slots.len() <= handle.pos {
        Some("handle pos out of bound")
      } else {
        let handle_dispatcher = handle
          .container_ref
          .upgrade()
          .expect("handle belongs to a disposed dispatcher");
        if !Arc::ptr_eq(&self.slots, &handle_dispatcher) {
          Some("handle does not belong to this dispatcher")
        } else {
          ::std::mem::replace(&mut slots[handle.pos], Slot::Empty);
          None
        }
      }
    };
    if let Some(msg) = fail {
      panic!("{}", msg);
    }
  }

  pub fn invoke(&self, arg: A) -> Vec<R>
  where
    A: Clone,
  {
    let lock = self.slots.read().unwrap();
    let mut pairs: Vec<_> = lock
      .iter()
      .filter_map(|slot| match *slot {
        Slot::Occupied(i, ref f) => Some((i, f.as_ref())),
        _ => None,
      })
      .collect();

    pairs.sort_by_key(|p| p.0);
    pairs.into_iter().map(|(_, f)| f(arg.clone())).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_delegate() {
    let d = Delegate::new();
    d.add(|x: i32| x + 1);
    let h2 = d.add(|x: i32| x + 2);
    d.add(|x: i32| x + 3);
    assert_eq!(d.invoke(0), vec![1, 2, 3]);

    d.remove(h2);
    assert_eq!(d.invoke(0), vec![1, 3]);
    assert_eq!(d.len(), 2);

    let h2 = d.add(|x: i32| x + 2);
    assert_eq!(h2.pos, 1);
    assert_eq!(d.invoke(0), vec![1, 3, 2]);
  }
}
