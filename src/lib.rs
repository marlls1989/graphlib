use slab::Slab;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
mod iterators;
use iterators::{LabelIter, VertexIter};

pub type VertexIndex = usize;
pub type EdgeIndex = (VertexIndex, VertexIndex);

#[derive(Default, Clone)]
struct Vertex<V: Hash + Eq + Clone> {
    pub preset: HashSet<VertexIndex>,
    pub posset: HashSet<VertexIndex>,
    pub aliases: HashSet<V>,
}

impl<V: Hash + Eq + Clone> Vertex<V> {
    pub fn new() -> Self {
        Vertex {
            preset: HashSet::new(),
            posset: HashSet::new(),
            aliases: HashSet::new(),
        }
    }

    #[inline]
    pub fn is_parallel(&self, other: &Self) -> bool {
        (self.preset.is_empty() && other.preset.is_empty()
            || !self.preset.is_disjoint(&other.preset))
            && (self.posset.is_empty() && other.posset.is_empty()
                || !self.posset.is_disjoint(&other.posset))
    }
}

pub struct Graph<V: Hash + Eq + Clone> {
    nodes: Slab<Vertex<V>>,
    trunks: HashSet<VertexIndex>,
    leaves: HashSet<VertexIndex>,
    aliases: HashMap<V, HashSet<VertexIndex>>,
}

impl<V: Eq + Hash + Clone> Graph<V> {
    #[inline]
    pub fn new() -> Self {
        Graph {
            nodes: Slab::new(),
            trunks: HashSet::new(),
            leaves: HashSet::new(),
            aliases: HashMap::new(),
        }
    }

    #[inline]
    pub fn insert(&mut self, label: V) -> VertexIndex {
        let node = Vertex::new();
        let index = self.nodes.insert(node);
        self.trunks.insert(index);
        self.leaves.insert(index);
        self.append_label(index, label);
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

    #[inline]
    pub fn remove(&mut self, vertex: VertexIndex) -> bool {
        if self.nodes.contains(vertex) {
            self.remove_vertex_node(vertex);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn posset<'a>(&'a self, vertex: VertexIndex) -> Option<VertexIter<'a>> {
        self.nodes
            .get(vertex)
            .map(|node| VertexIter::new(node.posset.iter()))
    }

    #[inline]
    pub fn preset<'a>(&'a self, vertex: VertexIndex) -> Option<VertexIter<'a>> {
        self.nodes
            .get(vertex)
            .map(|node| VertexIter::new(node.preset.iter()))
    }

    #[inline]
    pub fn indegree(&self, vertex: VertexIndex) -> Option<usize> {
        self.nodes.get(vertex).map(|node| node.preset.len())
    }

    #[inline]
    pub fn outdegree(&self, vertex: VertexIndex) -> Option<usize> {
        self.nodes.get(vertex).map(|node| node.posset.len())
    }

    #[inline]
    pub fn get<'a, 'b, W>(&'a self, label: &'b W) -> Option<VertexIter<'a>>
    where
        V: Borrow<W>,
        W: Eq + Hash + ?Sized,
    {
        self.aliases
            .get(label)
            .map(|set| VertexIter::new(set.iter()))
    }

    #[inline]
    pub fn labels<'a>(&'a self, vertex: VertexIndex) -> Option<LabelIter<'a, V>> {
        self.nodes
            .get(vertex)
            .map(|node| LabelIter::new(node.aliases.iter()))
    }

    #[inline]
    pub fn count_labeled<W: Borrow<V>>(&self, label: &W) -> Option<usize> {
        self.aliases.get(label.borrow()).map(|set| set.len())
    }

    #[inline]
    pub fn append_label(&mut self, vertex: VertexIndex, label: V) -> bool {
        let set = self.aliases.entry(label.clone()).or_default();

        match self.nodes.get_mut(vertex) {
            None => false,
            Some(node) => {
                node.aliases.insert(label.clone());
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

    #[inline]
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

    #[inline]
    pub fn trunks<'a>(&'a self) -> VertexIter<'a> {
        VertexIter::new(self.trunks.iter())
    }

    #[inline]
    pub fn leaves<'a>(&'a self) -> VertexIter<'a> {
        VertexIter::new(self.leaves.iter())
    }

    pub fn merge_vertices<'a, I>(&mut self, vertices: I) -> VertexIndex
    where
        I: IntoIterator<Item = VertexIndex>,
    {
        let mut posset = HashSet::new();
        let mut preset = HashSet::new();
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
            self.aliases.entry(label.clone()).or_default().insert(id);
        }

        let node = self.nodes.get_mut(id).unwrap();
        node.posset = posset;
        node.preset = preset;
        node.aliases = aliases;

        id
    }

    #[inline]
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
        let mut graph = Graph::new();
        let a = graph.insert("a");
        let b = graph.insert("b");
        let c = graph.insert("c");
        let d = graph.insert("d");
        let e = graph.insert("e");
        let f = graph.insert("f");
        let g = graph.insert("g");
        let h = graph.insert("h");
        let i = graph.insert("i");
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
        let a = graph.insert("a");
        let b = graph.insert("b");
        let c = graph.insert("c");
        let d = graph.insert("d");
        let e = graph.insert("e");
        let f = graph.insert("f");
        let g = graph.insert("g");
        let h = graph.insert("h");
        graph.connect(a, c);
        graph.connect(b, c);
        graph.connect(b, d);
        graph.connect(c, e);
        graph.connect(c, h);
        graph.connect(c, f);
        graph.connect(d, f);
        graph.connect(h, g);
        graph.connect(g, c);

        let a_pos: HashSet<VertexIndex> = graph.posset(a).unwrap().collect();
        let a_pre: HashSet<VertexIndex> = graph.preset(a).unwrap().collect();
        let labeled_a: HashSet<VertexIndex> = graph.get("a").unwrap().collect();
        assert_eq!(a_pre, vec![].into_iter().collect());
        assert_eq!(a_pos, vec![c].into_iter().collect());
        assert_eq!(labeled_a, vec![a].into_iter().collect());

        let b_pos: HashSet<VertexIndex> = graph.posset(b).unwrap().collect();
        let b_pre: HashSet<VertexIndex> = graph.preset(b).unwrap().collect();
        let labeled_b: HashSet<VertexIndex> = graph.get("b").unwrap().collect();
        assert_eq!(b_pre, vec![].into_iter().collect());
        assert_eq!(b_pos, vec![c, d].into_iter().collect());
        assert_eq!(labeled_b, vec![b].into_iter().collect());

        let c_pos: HashSet<VertexIndex> = graph.posset(c).unwrap().collect();
        let c_pre: HashSet<VertexIndex> = graph.preset(c).unwrap().collect();
        let labeled_c: HashSet<VertexIndex> = graph.get("c").unwrap().collect();
        assert_eq!(c_pre, vec![g, a, b].into_iter().collect());
        assert_eq!(c_pos, vec![e, f, h].into_iter().collect());
        assert_eq!(labeled_c, vec![c].into_iter().collect());

        let d_pos: HashSet<VertexIndex> = graph.posset(d).unwrap().collect();
        let d_pre: HashSet<VertexIndex> = graph.preset(d).unwrap().collect();
        let labeled_d: HashSet<VertexIndex> = graph.get("d").unwrap().collect();
        assert_eq!(d_pre, vec![b].into_iter().collect());
        assert_eq!(d_pos, vec![f].into_iter().collect());
        assert_eq!(labeled_d, vec![d].into_iter().collect());

        let e_pos: HashSet<VertexIndex> = graph.posset(e).unwrap().collect();
        let e_pre: HashSet<VertexIndex> = graph.preset(e).unwrap().collect();
        let labeled_e: HashSet<VertexIndex> = graph.get("e").unwrap().collect();
        assert_eq!(e_pos, vec![].into_iter().collect());
        assert_eq!(e_pre, vec![c].into_iter().collect());
        assert_eq!(labeled_e, vec![e].into_iter().collect());

        let f_pos: HashSet<VertexIndex> = graph.posset(f).unwrap().collect();
        let f_pre: HashSet<VertexIndex> = graph.preset(f).unwrap().collect();
        let labeled_f: HashSet<VertexIndex> = graph.get("f").unwrap().collect();
        assert_eq!(f_pos, vec![].into_iter().collect());
        assert_eq!(f_pre, vec![c, d].into_iter().collect());
        assert_eq!(labeled_f, vec![f].into_iter().collect());

        let h_pos: HashSet<VertexIndex> = graph.posset(h).unwrap().collect();
        let h_pre: HashSet<VertexIndex> = graph.preset(h).unwrap().collect();
        let labeled_h: HashSet<VertexIndex> = graph.get("h").unwrap().collect();
        assert_eq!(h_pos, vec![g].into_iter().collect());
        assert_eq!(h_pre, vec![c].into_iter().collect());
        assert_eq!(labeled_h, vec![h].into_iter().collect());

        let g_pos: HashSet<VertexIndex> = graph.posset(g).unwrap().collect();
        let g_pre: HashSet<VertexIndex> = graph.preset(g).unwrap().collect();
        let labeled_g: HashSet<VertexIndex> = graph.get("g").unwrap().collect();
        assert_eq!(g_pos, vec![c].into_iter().collect());
        assert_eq!(g_pre, vec![h].into_iter().collect());
        assert_eq!(labeled_g, vec![g].into_iter().collect());

        let ab = graph.merge_vertices(vec![a, b]);

        let ab_pos: HashSet<VertexIndex> = graph.posset(ab).unwrap().collect();
        let ab_pre: HashSet<VertexIndex> = graph.preset(ab).unwrap().collect();
        let labeled_a: HashSet<VertexIndex> = graph.get("a").unwrap().collect();
        let labeled_b: HashSet<VertexIndex> = graph.get("b").unwrap().collect();
        assert_eq!(ab_pre, vec![].into_iter().collect());
        assert_eq!(ab_pos, vec![c, d].into_iter().collect());
        assert_eq!(labeled_a, vec![ab].into_iter().collect());
        assert_eq!(labeled_b, vec![ab].into_iter().collect());

        let c_pos: HashSet<VertexIndex> = graph.posset(c).unwrap().collect();
        let c_pre: HashSet<VertexIndex> = graph.preset(c).unwrap().collect();
        let labeled_c: HashSet<VertexIndex> = graph.get("c").unwrap().collect();
        assert_eq!(c_pre, vec![g, ab].into_iter().collect());
        assert_eq!(c_pos, vec![e, f, h].into_iter().collect());
        assert_eq!(labeled_c, vec![c].into_iter().collect());

        let d_pos: HashSet<VertexIndex> = graph.posset(d).unwrap().collect();
        let d_pre: HashSet<VertexIndex> = graph.preset(d).unwrap().collect();
        let labeled_d: HashSet<VertexIndex> = graph.get("d").unwrap().collect();
        assert_eq!(d_pre, vec![ab].into_iter().collect());
        assert_eq!(d_pos, vec![f].into_iter().collect());
        assert_eq!(labeled_d, vec![d].into_iter().collect());

        let e_pos: HashSet<VertexIndex> = graph.posset(e).unwrap().collect();
        let e_pre: HashSet<VertexIndex> = graph.preset(e).unwrap().collect();
        let labeled_e: HashSet<VertexIndex> = graph.get("e").unwrap().collect();
        assert_eq!(e_pos, vec![].into_iter().collect());
        assert_eq!(e_pre, vec![c].into_iter().collect());
        assert_eq!(labeled_e, vec![e].into_iter().collect());

        let f_pos: HashSet<VertexIndex> = graph.posset(f).unwrap().collect();
        let f_pre: HashSet<VertexIndex> = graph.preset(f).unwrap().collect();
        let labeled_f: HashSet<VertexIndex> = graph.get("f").unwrap().collect();
        assert_eq!(f_pos, vec![].into_iter().collect());
        assert_eq!(f_pre, vec![c, d].into_iter().collect());
        assert_eq!(labeled_f, vec![f].into_iter().collect());

        let h_pos: HashSet<VertexIndex> = graph.posset(h).unwrap().collect();
        let h_pre: HashSet<VertexIndex> = graph.preset(h).unwrap().collect();
        let labeled_h: HashSet<VertexIndex> = graph.get("h").unwrap().collect();
        assert_eq!(h_pos, vec![g].into_iter().collect());
        assert_eq!(h_pre, vec![c].into_iter().collect());
        assert_eq!(labeled_h, vec![h].into_iter().collect());

        let g_pos: HashSet<VertexIndex> = graph.posset(g).unwrap().collect();
        let g_pre: HashSet<VertexIndex> = graph.preset(g).unwrap().collect();
        let labeled_g: HashSet<VertexIndex> = graph.get("g").unwrap().collect();
        assert_eq!(g_pos, vec![c].into_iter().collect());
        assert_eq!(g_pre, vec![h].into_iter().collect());
        assert_eq!(labeled_g, vec![g].into_iter().collect());

        let cd = graph.merge_vertices(vec![c, d]);

        let ab_pos: HashSet<VertexIndex> = graph.posset(ab).unwrap().collect();
        let ab_pre: HashSet<VertexIndex> = graph.preset(ab).unwrap().collect();
        let labeled_a: HashSet<VertexIndex> = graph.get("a").unwrap().collect();
        let labeled_b: HashSet<VertexIndex> = graph.get("b").unwrap().collect();
        assert_eq!(ab_pre, vec![].into_iter().collect());
        assert_eq!(ab_pos, vec![cd].into_iter().collect());
        assert_eq!(labeled_a, vec![ab].into_iter().collect());
        assert_eq!(labeled_b, vec![ab].into_iter().collect());

        let cd_pos: HashSet<VertexIndex> = graph.posset(cd).unwrap().collect();
        let cd_pre: HashSet<VertexIndex> = graph.preset(cd).unwrap().collect();
        let labeled_c: HashSet<VertexIndex> = graph.get("c").unwrap().collect();
        let labeled_d: HashSet<VertexIndex> = graph.get("d").unwrap().collect();
        assert_eq!(cd_pre, vec![g, ab].into_iter().collect());
        assert_eq!(cd_pos, vec![e, f, h].into_iter().collect());
        assert_eq!(labeled_c, vec![cd].into_iter().collect());
        assert_eq!(labeled_d, vec![cd].into_iter().collect());

        let e_pos: HashSet<VertexIndex> = graph.posset(e).unwrap().collect();
        let e_pre: HashSet<VertexIndex> = graph.preset(e).unwrap().collect();
        let labeled_e: HashSet<VertexIndex> = graph.get("e").unwrap().collect();
        assert_eq!(e_pos, vec![].into_iter().collect());
        assert_eq!(e_pre, vec![cd].into_iter().collect());
        assert_eq!(labeled_e, vec![e].into_iter().collect());

        let f_pos: HashSet<VertexIndex> = graph.posset(f).unwrap().collect();
        let f_pre: HashSet<VertexIndex> = graph.preset(f).unwrap().collect();
        let labeled_f: HashSet<VertexIndex> = graph.get("f").unwrap().collect();
        assert_eq!(f_pos, vec![].into_iter().collect());
        assert_eq!(f_pre, vec![cd].into_iter().collect());
        assert_eq!(labeled_f, vec![f].into_iter().collect());

        let h_pos: HashSet<VertexIndex> = graph.posset(h).unwrap().collect();
        let h_pre: HashSet<VertexIndex> = graph.preset(h).unwrap().collect();
        let labeled_h: HashSet<VertexIndex> = graph.get("h").unwrap().collect();
        assert_eq!(h_pos, vec![g].into_iter().collect());
        assert_eq!(h_pre, vec![cd].into_iter().collect());
        assert_eq!(labeled_h, vec![h].into_iter().collect());

        let g_pos: HashSet<VertexIndex> = graph.posset(g).unwrap().collect();
        let g_pre: HashSet<VertexIndex> = graph.preset(g).unwrap().collect();
        let labeled_g: HashSet<VertexIndex> = graph.get("g").unwrap().collect();
        assert_eq!(g_pos, vec![cd].into_iter().collect());
        assert_eq!(g_pre, vec![h].into_iter().collect());
        assert_eq!(labeled_g, vec![g].into_iter().collect());

        let ef = graph.merge_vertices(vec![e, f]);

        let ab_pos: HashSet<VertexIndex> = graph.posset(ab).unwrap().collect();
        let ab_pre: HashSet<VertexIndex> = graph.preset(ab).unwrap().collect();
        let labeled_a: HashSet<VertexIndex> = graph.get("a").unwrap().collect();
        let labeled_b: HashSet<VertexIndex> = graph.get("b").unwrap().collect();
        assert_eq!(ab_pre, vec![].into_iter().collect());
        assert_eq!(ab_pos, vec![cd].into_iter().collect());
        assert_eq!(labeled_a, vec![ab].into_iter().collect());
        assert_eq!(labeled_b, vec![ab].into_iter().collect());

        let cd_pos: HashSet<VertexIndex> = graph.posset(cd).unwrap().collect();
        let cd_pre: HashSet<VertexIndex> = graph.preset(cd).unwrap().collect();
        let labeled_c: HashSet<VertexIndex> = graph.get("c").unwrap().collect();
        let labeled_d: HashSet<VertexIndex> = graph.get("d").unwrap().collect();
        assert_eq!(cd_pre, vec![g, ab].into_iter().collect());
        assert_eq!(cd_pos, vec![ef, h].into_iter().collect());
        assert_eq!(labeled_c, vec![cd].into_iter().collect());
        assert_eq!(labeled_d, vec![cd].into_iter().collect());

        let ef_pos: HashSet<VertexIndex> = graph.posset(ef).unwrap().collect();
        let ef_pre: HashSet<VertexIndex> = graph.preset(ef).unwrap().collect();
        let labeled_e: HashSet<VertexIndex> = graph.get("e").unwrap().collect();
        let labeled_f: HashSet<VertexIndex> = graph.get("f").unwrap().collect();
        assert_eq!(ef_pos, vec![].into_iter().collect());
        assert_eq!(ef_pre, vec![cd].into_iter().collect());
        assert_eq!(labeled_e, vec![ef].into_iter().collect());
        assert_eq!(labeled_f, vec![ef].into_iter().collect());

        let h_pos: HashSet<VertexIndex> = graph.posset(h).unwrap().collect();
        let h_pre: HashSet<VertexIndex> = graph.preset(h).unwrap().collect();
        let labeled_h: HashSet<VertexIndex> = graph.get("h").unwrap().collect();
        assert_eq!(h_pos, vec![g].into_iter().collect());
        assert_eq!(h_pre, vec![cd].into_iter().collect());
        assert_eq!(labeled_h, vec![h].into_iter().collect());

        let g_pos: HashSet<VertexIndex> = graph.posset(g).unwrap().collect();
        let g_pre: HashSet<VertexIndex> = graph.preset(g).unwrap().collect();
        let labeled_g: HashSet<VertexIndex> = graph.get("g").unwrap().collect();
        assert_eq!(g_pos, vec![cd].into_iter().collect());
        assert_eq!(g_pre, vec![h].into_iter().collect());
        assert_eq!(labeled_g, vec![g].into_iter().collect());
    }

    #[test]
    fn connected_vertices() {
        let mut graph = Graph::new();
        let a = graph.insert("a");
        let b = graph.insert("b");
        let c = graph.insert("c");
        let d = graph.insert("d");
        graph.connect(a, b);
        graph.connect(b, c);
        graph.connect(b, d);
        graph.connect(d, c);

        let a_pos: HashSet<VertexIndex> = graph.posset(a).unwrap().collect();
        let b_pos: HashSet<VertexIndex> = graph.posset(b).unwrap().collect();
        let c_pos: HashSet<VertexIndex> = graph.posset(c).unwrap().collect();
        let d_pos: HashSet<VertexIndex> = graph.posset(d).unwrap().collect();
        let a_pre: HashSet<VertexIndex> = graph.preset(a).unwrap().collect();
        let b_pre: HashSet<VertexIndex> = graph.preset(b).unwrap().collect();
        let c_pre: HashSet<VertexIndex> = graph.preset(c).unwrap().collect();
        let d_pre: HashSet<VertexIndex> = graph.preset(d).unwrap().collect();

        assert_eq!(a_pos, vec![b].into_iter().collect());
        assert_eq!(b_pos, vec![c, d].into_iter().collect());
        assert_eq!(c_pos, vec![].into_iter().collect());
        assert_eq!(d_pos, vec![c].into_iter().collect());
        assert_eq!(a_pre, vec![].into_iter().collect());
        assert_eq!(b_pre, vec![a].into_iter().collect());
        assert_eq!(c_pre, vec![b, d].into_iter().collect());
        assert_eq!(d_pre, vec![b].into_iter().collect());
    }
}
