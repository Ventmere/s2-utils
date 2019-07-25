use std::vec::IntoIter;

#[allow(unused_imports)]
#[macro_use]
extern crate s2_utils_derive;
#[doc(hidden)]
pub use s2_utils_derive::*;

extern crate dotenv;

#[macro_use]
pub mod list;
pub mod delegate;
pub mod static_registry;
#[macro_use]
pub mod error;
pub mod env;

pub trait GroupByKey<T, I>: Sized + IntoIterator<Item = T>
where
    I: PartialEq + Ord,
{
    fn group_by_key<F>(self, f: F) -> IntoIter<(I, Vec<T>)>
    where
        F: Fn(&T) -> I,
    {
        let mut groups: Vec<(I, Vec<T>)> = vec![];
        for item in self.into_iter() {
            let id = f(&item);
            let pos = match groups.iter().position(|g| g.0 == id) {
                Some(pos) => pos,
                None => {
                    groups.push((id, vec![]));
                    groups.len() - 1
                }
            };
            groups[pos].1.push(item);
        }
        groups.into_iter()
    }
}

impl<T, Item, Id> GroupByKey<Item, Id> for T
where
    Id: PartialEq + Ord,
    T: IntoIterator<Item = Item>,
{
}
