use crate::VertexIndex;
use std::collections::hash_set;
use std::iter::Cloned;

#[derive(Clone, Debug)]
pub struct VertexIter<'a> {
  inner: Cloned<hash_set::Iter<'a, VertexIndex>>,
}

impl<'a> VertexIter<'a> {
  #[inline]
  pub(crate) fn new(inner: hash_set::Iter<'a, VertexIndex>) -> Self {
    VertexIter {
      inner: inner.cloned(),
    }
  }
}

impl<'a> Iterator for VertexIter<'a> {
  type Item = VertexIndex;

  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    self.inner.next()
  }
}

#[derive(Clone, Debug)]
pub struct LabelIter<'a, T: 'a> {
  inner: hash_set::Iter<'a, T>,
}

impl<'a, T> LabelIter<'a, T> {
  #[inline]
  pub(crate) fn new(inner: hash_set::Iter<'a, T>) -> Self {
    LabelIter { inner }
  }
}

impl<'a, T> Iterator for LabelIter<'a, T> {
  type Item = &'a T;

  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    self.inner.next()
  }
}
