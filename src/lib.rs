use slab::Slab;
use std::borrow::Borrow;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::iter::FromIterator;
use std::rc::Rc;

pub type VertexIndex = usize;
pub type EdgeIndex = (VertexIndex, VertexIndex);

#[derive(Default, Clone)]
struct Vertex<V: Hash + Eq> {
    pub preset: BTreeSet<VertexIndex>,
    pub posset: BTreeSet<VertexIndex>,
    pub aliases: HashSet<Rc<V>>,
}

impl<V> Vertex<V>
where
    V: Hash + Eq,
{
    pub fn new() -> Self {
        Vertex {
            preset: BTreeSet::new(),
            posset: BTreeSet::new(),
            aliases: HashSet::new(),
        }
    }

    pub fn is_parallel(&self, other: &Self) -> bool {
        (self.preset.is_empty() && other.preset.is_empty()
            || !self.preset.is_disjoint(&other.preset))
            && (self.posset.is_empty() && other.posset.is_empty()
                || !self.posset.is_disjoint(&other.posset))
    }
}

pub struct Graph<V>
where
    V: Hash + Eq,
{
    nodes: Slab<Vertex<V>>,
    trunks: BTreeSet<VertexIndex>,
    leaves: BTreeSet<VertexIndex>,
    aliases: HashMap<Rc<V>, BTreeSet<VertexIndex>>,
}

impl<V> Graph<V>
where
    V: Hash + Eq,
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

    pub fn count_labeled_vertices<W>(&self, label: &W) -> Option<usize>
    where
        W: Borrow<V>,
    {
        self.aliases.get(label.borrow()).map(|set| set.len())
    }

    pub fn append_vertex_label(&mut self, vertex: VertexIndex, label: V) -> bool {
        let label = match self.aliases.get_key_value(&label) {
            None => Rc::new(label),
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

    pub fn merge_vertices<'a, I>(&mut self, vertices: I) -> VertexIndex
    where
        I: IntoIterator<Item = VertexIndex>,
    {
        let mut posset = BTreeSet::new();
        let mut preset = BTreeSet::new();
        let mut aliases = HashSet::new();
        let mut reflexive = false;

        for vertex in vertices {
            let node = self.nodes.remove(vertex);

            for id in node.posset {
                if id != vertex {
                    posset.insert(id);
                    let other = self.nodes.get_mut(id).unwrap();
                    other.preset.remove(&vertex);
                } else {
                    reflexive = true;
                }
            }
            for id in node.preset {
                if id != vertex {
                    preset.insert(id);
                    let other = self.nodes.get_mut(id).unwrap();
                    other.posset.remove(&vertex);
                } else {
                    reflexive = true;
                }
            }
            for alias in node.aliases {
                let set = self.aliases.get_mut(&alias).unwrap();
                set.remove(&vertex);
                aliases.insert(alias);
            }
        }

        let id = self.nodes.insert(Vertex::new());

        if reflexive {
            posset.insert(id);
            preset.insert(id);
        }

        if !posset.is_empty() {
            self.leaves.remove(&id);
            for &dst in posset.iter() {
                self.trunks.remove(&dst);
                self.nodes.get_mut(dst).unwrap().preset.insert(id);
            }
        };

        if !preset.is_empty() {
            self.trunks.remove(&id);
            for &src in preset.iter() {
                self.leaves.remove(&src);
                self.nodes.get_mut(src).unwrap().posset.insert(id);
            }
        };

        for label in aliases.iter() {
            self.aliases.entry(Rc::clone(label)).or_default().insert(id);
        }

        let node = self.nodes.get_mut(id).unwrap();
        node.posset = posset;
        node.preset = preset;
        node.aliases = aliases;

        id
    }

    pub fn are_vertices_parallel(&self, one: VertexIndex, other: VertexIndex) -> Option<bool> {
        let one = self.nodes.get(one)?;
        let other = self.nodes.get(other)?;

        Some(one.is_parallel(other))
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn parallel_vertices() {
        let mut graph = Graph::<()>::new();
        let a = graph.new_vertex();
        let b = graph.new_vertex();
        let c = graph.new_vertex();
        let d = graph.new_vertex();
        let e = graph.new_vertex();
        let f = graph.new_vertex();
        let g = graph.new_vertex();
        let h = graph.new_vertex();
        let i = graph.new_vertex();
        graph.connect(a, c);
        graph.connect(b, c);
        graph.connect(b, d);
        graph.connect(c, e);
        graph.connect(c, h);
        graph.connect(c, f);
        graph.connect(d, f);
        graph.connect(h, g);
        graph.connect(g, c);
        graph.connect(d, i);
        assert_eq!(graph.are_vertices_parallel(b, a), Some(true));
        assert_eq!(graph.are_vertices_parallel(a, b), Some(true));
        assert_eq!(graph.are_vertices_parallel(e, f), Some(true));
        assert_eq!(graph.are_vertices_parallel(f, e), Some(true));
        assert_eq!(graph.are_vertices_parallel(c, d), Some(true));
        assert_eq!(graph.are_vertices_parallel(d, c), Some(true));

        assert_eq!(graph.are_vertices_parallel(h, g), Some(false));
        assert_eq!(graph.are_vertices_parallel(g, h), Some(false));
        assert_eq!(graph.are_vertices_parallel(d, f), Some(false));
        assert_eq!(graph.are_vertices_parallel(f, d), Some(false));
        assert_eq!(graph.are_vertices_parallel(b, d), Some(false));
        assert_eq!(graph.are_vertices_parallel(d, b), Some(false));
        assert_eq!(graph.are_vertices_parallel(b, f), Some(false));
        assert_eq!(graph.are_vertices_parallel(f, b), Some(false));
        assert_eq!(graph.are_vertices_parallel(h, f), Some(false));
        assert_eq!(graph.are_vertices_parallel(f, h), Some(false));
        assert_eq!(graph.are_vertices_parallel(e, h), Some(false));
        assert_eq!(graph.are_vertices_parallel(h, f), Some(false));
        assert_eq!(graph.are_vertices_parallel(d, e), Some(false));
        assert_eq!(graph.are_vertices_parallel(e, d), Some(false));
        assert_eq!(graph.are_vertices_parallel(g, a), Some(false));
        assert_eq!(graph.are_vertices_parallel(a, g), Some(false));
        assert_eq!(graph.are_vertices_parallel(i, h), Some(false));
        assert_eq!(graph.are_vertices_parallel(h, i), Some(false));
    }

    #[test]
    fn merge_vertices() {
        let mut graph = Graph::new();
        let a = graph.new_vertex();
        graph.append_vertex_label(a, "a".to_string());
        let b = graph.new_vertex();
        graph.append_vertex_label(b, "b".to_string());
        let c = graph.new_vertex();
        graph.append_vertex_label(c, "c".to_string());
        let d = graph.new_vertex();
        graph.append_vertex_label(d, "d".to_string());
        let e = graph.new_vertex();
        graph.append_vertex_label(e, "e".to_string());
        let f = graph.new_vertex();
        graph.append_vertex_label(f, "f".to_string());
        let g = graph.new_vertex();
        graph.append_vertex_label(g, "g".to_string());
        let h = graph.new_vertex();
        graph.append_vertex_label(h, "h".to_string());
        graph.connect(a, c);
        graph.connect(b, c);
        graph.connect(b, d);
        graph.connect(c, e);
        graph.connect(c, h);
        graph.connect(c, f);
        graph.connect(d, f);
        graph.connect(h, g);
        graph.connect(g, c);

        let ab = graph.merge_vertices(vec![a, b]);

        let ab_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(ab).unwrap();
        let ab_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(ab).unwrap();
        let labeled_a: BTreeSet<VertexIndex> =
            graph.collect_labeled_vertices(&"a".to_string()).unwrap();
        assert_eq!(ab_pre, vec![].into_iter().collect());
        assert_eq!(ab_pos, vec![c, d].into_iter().collect());

        let c_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(c).unwrap();
        let c_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(c).unwrap();
        // let c_label: BTreeSet<Rc<String>> = graph.collect_vertex_labels(ab).unwrap();
        assert_eq!(c_pre, vec![g, ab].into_iter().collect());
        assert_eq!(c_pos, vec![e, f, h].into_iter().collect());

        let d_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(d).unwrap();
        let d_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(d).unwrap();
        assert_eq!(d_pre, vec![ab].into_iter().collect());
        assert_eq!(d_pos, vec![f].into_iter().collect());

        let e_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(e).unwrap();
        let e_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(e).unwrap();
        assert_eq!(e_pos, vec![].into_iter().collect());
        assert_eq!(e_pre, vec![c].into_iter().collect());

        let f_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(f).unwrap();
        let f_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(f).unwrap();
        assert_eq!(f_pos, vec![].into_iter().collect());
        assert_eq!(f_pre, vec![c, d].into_iter().collect());

        let h_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(h).unwrap();
        let h_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(h).unwrap();
        assert_eq!(h_pos, vec![g].into_iter().collect());
        assert_eq!(h_pre, vec![c].into_iter().collect());

        let g_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(g).unwrap();
        let g_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(g).unwrap();
        assert_eq!(g_pos, vec![c].into_iter().collect());
        assert_eq!(g_pre, vec![h].into_iter().collect());

        let cd = graph.merge_vertices(vec![c, d]);

        let ab_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(ab).unwrap();
        let ab_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(ab).unwrap();
        assert_eq!(ab_pre, vec![].into_iter().collect());
        assert_eq!(ab_pos, vec![cd].into_iter().collect());

        let cd_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(cd).unwrap();
        let cd_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(cd).unwrap();
        assert_eq!(cd_pre, vec![g, ab].into_iter().collect());
        assert_eq!(cd_pos, vec![e, f, h].into_iter().collect());

        let e_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(e).unwrap();
        let e_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(e).unwrap();
        assert_eq!(e_pos, vec![].into_iter().collect());
        assert_eq!(e_pre, vec![cd].into_iter().collect());

        let f_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(f).unwrap();
        let f_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(f).unwrap();
        assert_eq!(f_pos, vec![].into_iter().collect());
        assert_eq!(f_pre, vec![cd].into_iter().collect());

        let h_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(h).unwrap();
        let h_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(h).unwrap();
        assert_eq!(h_pos, vec![g].into_iter().collect());
        assert_eq!(h_pre, vec![cd].into_iter().collect());

        let g_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(g).unwrap();
        let g_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(g).unwrap();
        assert_eq!(g_pos, vec![cd].into_iter().collect());
        assert_eq!(g_pre, vec![h].into_iter().collect());

        let ef = graph.merge_vertices(vec![e, f]);

        let ab_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(ab).unwrap();
        let ab_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(ab).unwrap();
        assert_eq!(ab_pos, vec![cd].into_iter().collect());
        assert_eq!(ab_pre, vec![].into_iter().collect());

        let cd_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(cd).unwrap();
        let cd_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(cd).unwrap();
        assert_eq!(cd_pos, vec![h, ef].into_iter().collect());
        assert_eq!(cd_pre, vec![ab, g].into_iter().collect());

        let ef_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(ef).unwrap();
        let ef_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(ef).unwrap();
        assert_eq!(ef_pos, vec![].into_iter().collect());
        assert_eq!(ef_pre, vec![cd].into_iter().collect());

        let h_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(h).unwrap();
        let h_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(h).unwrap();
        assert_eq!(h_pos, vec![g].into_iter().collect());
        assert_eq!(h_pre, vec![cd].into_iter().collect());

        let g_pos: BTreeSet<VertexIndex> = graph.collect_vertex_posset(g).unwrap();
        let g_pre: BTreeSet<VertexIndex> = graph.collect_vertex_preset(g).unwrap();
        assert_eq!(g_pos, vec![cd].into_iter().collect());
        assert_eq!(g_pre, vec![h].into_iter().collect());
    }

    #[test]
    fn connected_vertices() {
        let mut graph = Graph::<()>::new();
        let a = graph.new_vertex();
        let b = graph.new_vertex();
        let c = graph.new_vertex();
        let d = graph.new_vertex();
        graph.connect(a, b);
        graph.connect(b, c);
        graph.connect(b, d);
        graph.connect(d, c);

        let a_pos: Vec<VertexIndex> = graph.collect_vertex_posset(a).unwrap();
        let b_pos: Vec<VertexIndex> = graph.collect_vertex_posset(b).unwrap();
        let c_pos: Vec<VertexIndex> = graph.collect_vertex_posset(c).unwrap();
        let d_pos: Vec<VertexIndex> = graph.collect_vertex_posset(d).unwrap();
        let a_pre: Vec<VertexIndex> = graph.collect_vertex_preset(a).unwrap();
        let b_pre: Vec<VertexIndex> = graph.collect_vertex_preset(b).unwrap();
        let c_pre: Vec<VertexIndex> = graph.collect_vertex_preset(c).unwrap();
        let d_pre: Vec<VertexIndex> = graph.collect_vertex_preset(d).unwrap();

        assert_eq!(a_pos, vec![b]);
        assert_eq!(b_pos, vec![c, d]);
        assert_eq!(c_pos, vec![]);
        assert_eq!(d_pos, vec![c]);
        assert_eq!(a_pre, vec![]);
        assert_eq!(b_pre, vec![a]);
        assert_eq!(c_pre, vec![b, d]);
        assert_eq!(d_pre, vec![b]);
    }
}
