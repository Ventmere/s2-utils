pub use super::GroupByKey;

pub trait HasItemKey<K> {
  fn get_item_key(&self) -> K;
}

impl<T, K> HasItemKey<K> for (usize, T)
where
  T: HasItemKey<K>,
{
  fn get_item_key(&self) -> K {
    self.1.get_item_key()
  }
}

impl<'a, T, K> HasItemKey<K> for &'a T
where
  T: HasItemKey<K>,
{
  fn get_item_key(&self) -> K {
    (self as &T).get_item_key()
  }
}

impl HasItemKey<i32> for i32 {
  fn get_item_key(&self) -> i32 {
    *self
  }
}

impl HasItemKey<String> for String {
  fn get_item_key(&self) -> String {
    self.clone()
  }
}

#[macro_export]
macro_rules! impl_has_item_key {
  (| $s:ident : & $t:ty | -> $k:ty { $expr:expr }) => {
    impl $crate::list::HasItemKey<$k> for $t {
      fn get_item_key(&self) -> $k {
        let $s = self;
        $expr
      }
    }
  };
}

pub struct ChangedItems<'a, TE, TN>
where
  TE: 'a,
  TN: 'a,
{
  pub add: Vec<&'a TN>,
  pub update: Vec<(&'a TE, &'a TN)>,
  pub delete: Vec<&'a TE>,
}

pub fn get_changed_items<'a, K, TE, TN>(
  existing_items: &'a Vec<TE>,
  new_items: &'a Vec<TN>,
) -> ChangedItems<'a, TE, TN>
where
  K: PartialEq,
  TE: HasItemKey<K> + 'a,
  TN: HasItemKey<K> + 'a,
{
  let mut result = ChangedItems {
    add: vec![],
    update: vec![],
    delete: vec![],
  };

  for ni in new_items {
    let key = ni.get_item_key();
    let existing = existing_items.iter().find(|i| i.get_item_key() == key);
    match existing {
      Some(ei) => result.update.push((ei, ni)),
      None => result.add.push(ni),
    }
  }

  for ei in existing_items {
    let key = ei.get_item_key();
    let new = new_items.iter().find(|i| i.get_item_key() == key);
    match new {
      Some(_) => {}
      None => result.delete.push(ei),
    }
  }

  result
}

pub fn get_dup_items<'a, K, T>(items: &'a Vec<T>) -> Vec<(K, Vec<(usize, &'a T)>)>
where
  K: PartialEq,
  T: HasItemKey<K> + 'a,
{
  let mut key_items: Vec<(K, Vec<usize>)> = vec![];
  for (idx, item) in items.iter().enumerate() {
    let key = item.get_item_key();
    match key_items.iter().position(|i| i.0 == key) {
      Some(pos) => key_items[pos].1.push(idx),
      None => key_items.push((key, vec![idx])),
    }
  }
  key_items
    .into_iter()
    .filter_map(|(k, idxs)| {
      if idxs.len() > 1 {
        Some((k, idxs.into_iter().map(|idx| (idx, &items[idx])).collect()))
      } else {
        None
      }
    }).collect()
}

pub enum DedupKeyList<T> {
  Empty,
  One(T),
  Many(Vec<T>),
}

impl<T> DedupKeyList<T> {
  fn len(&self) -> usize {
    match *self {
      DedupKeyList::Empty => 0,
      DedupKeyList::One(_) => 1,
      DedupKeyList::Many(ref v) => v.len(),
    }
  }

  fn move_to(self, out: &mut Vec<T>) {
    match self {
      DedupKeyList::Empty => {}
      DedupKeyList::One(item) => out.push(item),
      DedupKeyList::Many(mut items) => out.append(&mut items),
    }
  }
}

impl<T> From<T> for DedupKeyList<T> {
  fn from(v: T) -> DedupKeyList<T> {
    DedupKeyList::One(v)
  }
}

impl<T> From<Option<T>> for DedupKeyList<T> {
  fn from(v: Option<T>) -> DedupKeyList<T> {
    match v {
      Some(v) => DedupKeyList::One(v),
      None => DedupKeyList::Empty,
    }
  }
}

impl<T> From<Vec<T>> for DedupKeyList<T> {
  fn from(v: Vec<T>) -> DedupKeyList<T> {
    DedupKeyList::Many(v)
  }
}

pub fn dedup_map<'a, I, F, R, RI>(items: &'a [I], f: F) -> Vec<RI>
where
  F: Fn(&'a I) -> R,
  R: Into<DedupKeyList<RI>>,
  RI: PartialEq<RI> + Ord,
{
  let kls: Vec<_> = items
    .into_iter()
    .map(|item| -> DedupKeyList<RI> { f(item).into() })
    .collect();
  let len = kls.iter().map(DedupKeyList::len).sum();
  let mut all_items = Vec::with_capacity(len);
  for kl in kls {
    kl.move_to(&mut all_items);
  }
  all_items.sort();
  all_items.dedup();
  all_items
}

impl<'a, T, K> HasItemKey<K> for (&'a T, K)
where
  K: Clone,
{
  fn get_item_key(&self) -> K {
    self.1.clone()
  }
}

pub fn with_item_keys<T, K, F>(list: &[T], f: F) -> Vec<(&T, K)>
where
  F: Fn(&T) -> K,
{
  list.iter().map(|i| (i, f(i))).collect()
}
