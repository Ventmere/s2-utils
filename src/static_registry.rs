use std::sync::RwLock;

pub struct StaticRegistry<T: Sync + 'static> {
  nodes: RwLock<Vec<T>>,
}

impl<T: Sync + 'static> StaticRegistry<T> {
  pub fn new() -> Self {
    Self {
      nodes: RwLock::new(vec![]),
    }
  }

  pub fn register(&self, node: T) {
    let mut lock = self.nodes.write().unwrap();
    lock.push(node);
  }

  pub fn with_nodes<F, R>(&self, f: F) -> R
  where
    F: FnOnce(&Vec<T>) -> R,
  {
    let lock = self.nodes.read().unwrap();
    f(&lock)
  }
}
