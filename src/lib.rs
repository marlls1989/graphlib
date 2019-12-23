use slab::Slab;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::iter::FromIterator;
use std::rc::Rc;

pub type VertexIndex = usize;
pub type EdgeIndex = (VertexIndex, VertexIndex);

#[derive(Default, Clone)]
struct Vertex<V>
where
    V: Hash + Eq + Sized + Clone,
{
    pub preset: BTreeSet<VertexIndex>,
    pub posset: BTreeSet<VertexIndex>,
    pub aliases: HashSet<Rc<V>>,
}

impl<V> Vertex<V>
where
    V: Hash + Eq + Sized + Clone,
{
    pub fn new() -> Self {
        Vertex {
            preset: BTreeSet::new(),
            posset: BTreeSet::new(),
            aliases: HashSet::new(),
        }
    }
}

#[derive(Clone)]
pub struct Graph<V>
where
    V: Hash + Eq + Sized + Clone,
{
    nodes: Slab<Vertex<V>>,
    trunks: BTreeSet<VertexIndex>,
    leaves: BTreeSet<VertexIndex>,
    aliases: HashMap<Rc<V>, BTreeSet<VertexIndex>>,
}

impl<V> Graph<V>
where
    V: Hash + Eq + Sized + Clone,
{
    pub fn new() -> Self {
        Graph {
            nodes: Slab::new(),
            trunks: BTreeSet::new(),
            leaves: BTreeSet::new(),
            aliases: HashMap::new(),
        }
    }

    pub fn new_vertex(&mut self) -> VertexIndex {
        let node = Vertex::new();
        let index = self.nodes.insert(node);
        self.trunks.insert(index);
        self.leaves.insert(index);
        index
    }

    fn remove_vertex_node(&mut self, vertex: VertexIndex) -> Vertex<V> {
        let node = self.nodes.get(vertex).unwrap();
        let posset: Vec<VertexIndex> = node.posset.iter().cloned().collect();
        let preset: Vec<VertexIndex> = node.posset.iter().cloned().collect();

        for dst in posset {
            self.disconnect((vertex, dst));
        }

        for src in preset {
            self.disconnect((src, vertex));
        }

        self.trunks.remove(&vertex);
        self.leaves.remove(&vertex);

        let node = self.nodes.remove(vertex);
        for id in node.aliases.iter() {
            let set = self.aliases.get_mut(id).unwrap();
            set.remove(&vertex);
            if set.is_empty() {
                self.aliases.remove(id);
            }
        }

        node
    }

    pub fn remove_vertex(&mut self, vertex: VertexIndex) -> bool {
        if self.nodes.contains(vertex) {
            self.remove_vertex_node(vertex);
            true
        } else {
            false
        }
    }

    pub fn collect_vertex_posset<B>(&self, vertex: VertexIndex) -> Option<B>
    where
        B: FromIterator<VertexIndex>,
    {
        self.nodes
            .get(vertex)
            .map(|node| B::from_iter(node.posset.iter().cloned()))
    }

    pub fn collect_vertex_preset<B>(&self, vertex: VertexIndex) -> Option<B>
    where
        B: FromIterator<VertexIndex>,
    {
        self.nodes
            .get(vertex)
            .map(|node| B::from_iter(node.preset.iter().cloned()))
    }

    pub fn vertex_indegree(&self, vertex: VertexIndex) -> Option<usize> {
        self.nodes.get(vertex).map(|node| node.preset.len())
    }

    pub fn vertex_outdegree(&self, vertex: VertexIndex) -> Option<usize> {
        self.nodes.get(vertex).map(|node| node.posset.len())
    }

    pub fn collect_labeled_vertices<B>(&self, label: &V) -> Option<B>
    where
        B: FromIterator<VertexIndex>,
    {
        self.aliases
            .get(label)
            .map(|set| B::from_iter(set.iter().cloned()))
    }

    pub fn count_labeled_vertices<B>(&self, label: &V) -> Option<usize> {
        self.aliases.get(label).map(|set| set.len())
    }

    pub fn append_vertex_label(&mut self, label: &V, vertex: VertexIndex) -> bool {
        let label = match self.aliases.get_key_value(label) {
            None => Rc::new(label.clone()),
            Some((key, _)) => Rc::clone(key),
        };
        let set = self.aliases.entry(Rc::clone(&label)).or_default();

        match self.nodes.get_mut(vertex) {
            None => false,
            Some(node) => {
                node.aliases.insert(Rc::clone(&label));
                set.insert(vertex);
                true
            }
        }
    }

    pub fn remove_vertex_label(&mut self, label: &V, vertex: VertexIndex) -> bool {
        let node = match self.nodes.get_mut(vertex) {
            None => return false,
            Some(node) => node,
        };
        node.aliases.remove(label);

        let set = match self.aliases.get_mut(label) {
            None => return false,
            Some(set) => set,
        };
        set.remove(&vertex);
        if set.len() == 0 {
            self.aliases.remove(label);
        }

        true
    }

    pub fn connect(&mut self, src: VertexIndex, dst: VertexIndex) -> Option<EdgeIndex> {
        if !(self.nodes.contains(src) && self.nodes.contains(dst)) {
            return None;
        }

        self.nodes.get_mut(src).unwrap().posset.insert(dst);
        self.nodes.get_mut(dst).unwrap().preset.insert(src);
        self.trunks.remove(&dst);
        self.leaves.remove(&src);

        Some((src, dst))
    }

    pub fn disconnect(&mut self, edge: EdgeIndex) -> bool {
        let (src, dst) = edge;
        if !(self.nodes.contains(src) && self.nodes.contains(dst)) {
            return false;
        }

        let src_node = self.nodes.get_mut(src).unwrap();
        if !src_node.posset.remove(&dst) {
            return false;
        }
        if src_node.posset.is_empty() {
            self.leaves.insert(src);
        }

        let dst_node = self.nodes.get_mut(dst).unwrap();
        if !dst_node.preset.remove(&src) {
            return false;
        }
        if dst_node.preset.is_empty() {
            self.trunks.insert(dst);
        }

        true
    }

    pub fn collect_trunks<B>(&self) -> B
    where
        B: FromIterator<VertexIndex>,
    {
        B::from_iter(self.trunks.iter().cloned())
    }

    pub fn collect_leaves<B>(&self) -> B
    where
        B: FromIterator<VertexIndex>,
    {
        B::from_iter(self.leaves.iter().cloned())
    }

    pub fn merge_vertices<'a, I>(&mut self, vertices: I)
    where
        I: IntoIterator<Item = VertexIndex>,
    {
        let mut posset = BTreeSet::new();
        let mut merged = BTreeSet::new();
        let mut preset = BTreeSet::new();
        let mut aliases = HashSet::new();

        for vertex in vertices {
            merged.insert(vertex);
            let mut node = self.remove_vertex_node(vertex);
            posset.append(&mut node.posset);
            preset.append(&mut node.preset);
            aliases.extend(node.aliases);
        }

        let posset: BTreeSet<VertexIndex> = posset.difference(&merged).cloned().collect();
        let preset: BTreeSet<VertexIndex> = preset.difference(&merged).cloned().collect();
        let id = self.nodes.insert(Vertex::new());

        if !posset.is_empty() {
            self.leaves.remove(&id);
            for &dst in posset.iter() {
                self.trunks.remove(&dst);
                self.nodes.get_mut(dst).unwrap().posset.insert(id);
            }
        };

        if !preset.is_empty() {
            self.trunks.remove(&id);
            for &src in posset.iter() {
                self.leaves.remove(&src);
                self.nodes.get_mut(src).unwrap().preset.insert(id);
            }
        };

        for label in aliases.iter() {
            self.aliases.entry(Rc::clone(label)).or_default().insert(id);
        }

        let node = self.nodes.get_mut(id).unwrap();
        node.posset = posset;
        node.preset = preset;
        node.aliases = aliases;
    }

    pub fn are_vertices_parallel(&self, src: VertexIndex, dst: VertexIndex) -> bool {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
