use std::{
    collections::HashMap,
    fmt::{self, Debug, Formatter},
    hash::Hash,
    iter::FromIterator,
    marker::PhantomData,
    sync::Arc,
};

use itertools::Itertools;

#[derive(Clone, Eq, PartialEq)]
pub struct HierarchyTree<T> where T: Hash + Eq {
    hierarchy: HashMap<usize, Vec<Arc<T>>>,
    rev_hierarchy: HashMap<Vec<Arc<T>>, usize>,
}

impl<T> Debug for HierarchyTree<T> where T: Debug + Hash + Eq {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Hierarchy: {:?}", self.hierarchy)
    }
}

impl<T> HierarchyTree<T> where T: Hash + Eq {
    pub fn new(elements: Vec<T>) -> HierarchyTree<T> {
        let elements = elements
            .into_iter()
            .map(Arc::new)
            .collect_vec();
        let hierarchy = HashMap::from_iter((0..elements.len() + 1)
            .rev()
            .flat_map(move |n| elements
                .clone()
                .into_iter()
                .combinations(n)
                .collect_vec())
            .enumerate());
        let rev_hierarchy = HashMap::<Vec<Arc<T>>, usize>::from_iter(hierarchy
            .iter()
            .map(|(k, v)| ((v.clone()), (k.clone()))));
        HierarchyTree { hierarchy, rev_hierarchy }
    }

    pub fn top_iter(&self) -> Iter<T> {
        Iter { tree: self, position: 0, item_type: Default::default() }
    }

    pub fn bottom_iter(&self) -> Iter<T> {
        Iter { tree: self, position: std::usize::MAX, item_type: Default::default() }
    }

    pub fn idx_to_vec(&self, n: usize) -> Option<Vec<Arc<T>>> {
        self.hierarchy.get(&n).map(|v| v.clone())
    }

    pub fn vec_to_idx(&self, v: &Vec<Arc<T>>) -> Option<usize> {
        self.rev_hierarchy.get(v).map(|v| v.clone())
    }
}

pub struct Iter<'a, T> where T: Hash + Eq {
    tree: &'a HierarchyTree<T>,
    position: usize,
    item_type: PhantomData<T>,
}

impl<'a, T> Iter<'a, T> where T: Hash + Eq {
    pub fn current_vec(&self) -> Vec<Arc<T>> {
        self.tree
            .idx_to_vec(self.position.clone())
            .unwrap_or(Vec::new())
    }

    pub fn current_idx(&self) -> &usize {
        &self.position
    }

    pub fn children(&self) -> Vec<Vec<Arc<T>>> {
        let elements = self.current_vec().clone();
        let n = elements.len();
        elements.into_iter()
            .combinations(n - 1)
            .map(|v| v.into_iter().map(|v| v.clone()).collect_vec())
            .collect_vec()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let elements = vec![1, 2, 3, 4];
        let tree: HierarchyTree<i32> = HierarchyTree::new(elements.clone());

        println!("Hierarchy tree: {:?}", tree);

        // First combination
        for i in 0..tree.hierarchy[&0].len() {
            assert_eq!(*tree.hierarchy[&0][i], elements[i]);
        }

        // Last combination
        assert_eq!(tree.hierarchy[&(tree.hierarchy.len() - 1)].len(), 0);
    }

    #[test]
    fn top_iter() {
        let elements = vec![1, 2, 3, 4];
        let tree: HierarchyTree<i32> = HierarchyTree::new(elements.clone());

        let iter: Iter<i32> = tree.top_iter();

        let first_elements: Vec<Arc<i32>> = iter.current_vec();
        assert_eq!(first_elements.into_iter().map(|v| *v).collect_vec(), elements);

        let children: Vec<Vec<Arc<i32>>> = iter.children();
        println!("Children: {:?}", &children);
        let mut child_iter = children.into_iter();
        assert_eq!(child_iter.next().map(|v| v.into_iter()
            .map(|v| *v).collect_vec()), Some(vec![1, 2, 3]));
        assert_eq!(child_iter.next().map(|v| v.into_iter()
            .map(|v| *v).collect_vec()), Some(vec![1, 2, 4]));
        assert_eq!(child_iter.next().map(|v| v.into_iter()
            .map(|v| *v).collect_vec()), Some(vec![1, 3, 4]));
        assert_eq!(child_iter.next().map(|v| v.into_iter()
            .map(|v| *v).collect_vec()), Some(vec![2, 3, 4]));
        assert_eq!(child_iter.next().map(|v| v.into_iter()
            .map(|v| *v).collect_vec()), None);
    }

    #[test]
    fn bottom_iter() {}
}

