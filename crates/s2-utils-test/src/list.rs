use s2_utils::list::get_changed_items;
use s2_utils_derive::HasItemKey;

#[derive(Debug, HasItemKey, PartialEq)]
#[has_item_key(i32, expr = "self.0")]
struct A(i32);

#[derive(Debug, HasItemKey, PartialEq)]
#[has_item_key(i32, expr = "self.0")]
struct B(i32);

#[test]
fn test_get_item_changes() {
  {
    let l1 = vec![A(1), A(2), A(3)];
    let l2 = vec![B(1), B(2), B(3)];

    let changes = get_changed_items(&l1, &l2);
    assert!(changes.add.is_empty());
    assert!(changes.delete.is_empty());
    assert_eq!(
      changes.update,
      vec![(&A(1), &B(1)), (&A(2), &B(2)), (&A(3), &B(3)),]
    )
  }

  {
    let l1 = vec![A(1), A(3)];
    let l2 = vec![B(1), B(2), B(3)];

    let changes = get_changed_items(&l1, &l2);
    assert_eq!(changes.add, vec![&B(2)]);
    assert!(changes.delete.is_empty());
    assert_eq!(changes.update, vec![(&A(1), &B(1)), (&A(3), &B(3)),])
  }

  {
    let l1 = vec![A(1), A(2), A(3)];
    let l2 = vec![B(1), B(3)];

    let changes = get_changed_items(&l1, &l2);
    assert!(changes.add.is_empty());
    assert_eq!(changes.delete, vec![&A(2)]);
    assert_eq!(changes.update, vec![(&A(1), &B(1)), (&A(3), &B(3)),])
  }

  // dup existing items
  {
    let l1 = vec![A(1), A(2), A(2), A(3)];
    let l2 = vec![B(1), B(3)];

    let changes = get_changed_items(&l1, &l2);
    assert!(changes.add.is_empty());
    assert_eq!(changes.delete, vec![&A(2), &A(2)]);
    assert_eq!(changes.update, vec![(&A(1), &B(1)), (&A(3), &B(3)),])
  }

  // dup existing items, update 1, delete rest
  {
    let l1 = vec![A(1), A(2), A(2), A(3)];
    let l2 = vec![B(1), B(2), B(3)];

    let changes = get_changed_items(&l1, &l2);
    assert!(changes.add.is_empty());
    assert_eq!(changes.delete, vec![&A(2)]);
    assert_eq!(
      changes.update,
      vec![(&A(1), &B(1)), (&A(2), &B(2)), (&A(3), &B(3)),]
    )
  }

  // dup new items
  {
    let l1 = vec![A(1), A(2), A(2), A(3)];
    let l2 = vec![B(1), B(3), B(3)];

    let changes = get_changed_items(&l1, &l2);
    assert!(changes.add.is_empty());
    assert_eq!(changes.delete, vec![&A(2), &A(2)]);
    assert_eq!(changes.update, vec![(&A(1), &B(1)), (&A(3), &B(3)),])
  }
}
