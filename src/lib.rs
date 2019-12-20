use slab::*;
use std::collections::*;
use std::rc::*;

struct Node<V> {
    preset: HashSet<usize>,
    posset: HashSet<usize>,
    aliases: HashSet<Rc<V>>,
}

pub struct Graph<V> {
    nodes: Slab<Node<V>>,
    aliases: HashMap<Rc<V>, HashSet<usize>>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
