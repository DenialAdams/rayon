#![feature(conservative_impl_trait)]

extern crate rayon;

use rayon::prelude::*;
use rayon::iter::internal::*;

/// Stress-test indexes for `Producer::split_at`.
fn check<F, I>(expected: &[I::Item], mut f: F)
    where F: FnMut() -> I,
          I: IntoParallelIterator,
          I::Iter: IndexedParallelIterator,
          I::Item: PartialEq + std::fmt::Debug
{
    for (i, j, k) in triples(expected.len() + 1) {
        Split::forward(f(), i, j, k, expected);
        Split::reverse(f(), i, j, k, expected);
    }
}

fn triples(end: usize) -> impl Iterator<Item=(usize, usize, usize)> {
    (0..end).flat_map(move |i| {
        (i..end).flat_map(move |j| {
            (j..end).map(move |k| (i, j, k))
        })
    })
}

#[derive(Debug)]
struct Split {
    i: usize,
    j: usize,
    k: usize,
    reverse: bool
}

impl Split {
    fn forward<I>(iter: I, i: usize, j: usize, k: usize, expected: &[I::Item])
        where I: IntoParallelIterator,
              I::Iter: IndexedParallelIterator,
              I::Item: PartialEq + std::fmt::Debug
    {
        let result = iter.into_par_iter()
            .with_producer(Split { i, j, k, reverse: false });
        assert_eq!(result, expected);
    }

    fn reverse<I>(iter: I, i: usize, j: usize, k: usize, expected: &[I::Item])
        where I: IntoParallelIterator,
              I::Iter: IndexedParallelIterator,
              I::Item: PartialEq + std::fmt::Debug
    {
        let result = iter.into_par_iter()
            .with_producer(Split { i, j, k, reverse: true });
        assert!(result.iter().eq(expected.iter().rev()));
    }
}

impl<T> ProducerCallback<T> for Split {
    type Output = Vec<T>;

    fn callback<P>(self, producer: P) -> Self::Output
        where P: Producer<Item = T>
    {
        println!("{:?}", self);

        // Splitting the outer indexes first gets us an arbitrary mid section,
        // which we then split further to get full test coverage.
        let (left, d) = producer.split_at(self.k);
        let (a, mid) = left.split_at(self.i);
        let (b, c) = mid.split_at(self.j - self.i);

        let a = a.into_iter();
        let b = b.into_iter();
        let c = c.into_iter();
        let d = d.into_iter();

        check_len(&a, self.i);
        check_len(&b, self.j - self.i);
        check_len(&c, self.k - self.j);

        let chain = a.chain(b).chain(c).chain(d);
        if self.reverse {
            chain.rev().collect()
        } else {
            chain.collect()
        }
    }
}

fn check_len<I: ExactSizeIterator>(iter: &I, len: usize) {
    assert_eq!(iter.size_hint(), (len, Some(len)));
    assert_eq!(iter.len(), len);
}


// **** Base Producers ****

#[test]
fn option() {
    let v = vec![42];
    check(&v, || Some(42));
}

#[test]
fn range() {
    let v: Vec<_> = (0..10).collect();
    check(&v, || 0..10);
}

#[test]
fn repeatn() {
    let v: Vec<_> = std::iter::repeat(1).take(5).collect();
    check(&v, || rayon::iter::repeatn(1, 5));
}

#[test]
fn slice_iter() {
    let s: Vec<_> = (0..10).collect();
    let v: Vec<_> = s.iter().collect();
    check(&v, || &s);
}

#[test]
fn slice_iter_mut() {
    let mut s: Vec<_> = (0..10).collect();
    let mut v: Vec<_> = s.clone();
    let expected: Vec<_> = v.iter_mut().collect();

    for (i, j, k) in triples(expected.len() + 1) {
        Split::forward(s.par_iter_mut(), i, j, k, &expected);
        Split::reverse(s.par_iter_mut(), i, j, k, &expected);
    }
}

#[test]
fn slice_chunks() {
    let s: Vec<_> = (0..10).collect();
    let v: Vec<_> = s.chunks(2).collect();
    check(&v, || s.par_chunks(2));
}

#[test]
fn slice_chunks_mut() {
    let mut s: Vec<_> = (0..10).collect();
    let mut v: Vec<_> = s.clone();
    let expected: Vec<_> = v.chunks_mut(2).collect();

    for (i, j, k) in triples(expected.len() + 1) {
        Split::forward(s.par_chunks_mut(2), i, j, k, &expected);
        Split::reverse(s.par_chunks_mut(2), i, j, k, &expected);
    }
}

#[test]
fn slice_windows() {
    let s: Vec<_> = (0..10).collect();
    let v: Vec<_> = s.windows(2).collect();
    check(&v, || s.par_windows(2));
}

#[test]
fn vec() {
    let v: Vec<_> = (0..10).collect();
    check(&v, || v.clone());
}


// **** Adaptors ****

#[test]
fn chain() {
    let v: Vec<_> = (0..10).collect();
    check(&v, || (0..5).into_par_iter().chain(5..10));
}

#[test]
fn cloned() {
    let v: Vec<_> = (0..10).collect();
    check(&v, || v.par_iter().cloned());
}

#[test]
fn enumerate() {
    let v: Vec<_> = (0..10).enumerate().collect();
    check(&v, || (0..10).into_par_iter().enumerate());
}

#[test]
fn inspect() {
    let v: Vec<_> = (0..10).collect();
    check(&v, || (0..10).into_par_iter().inspect(|_| ()));
}

#[test]
fn interleave() {
    let v = [0, 10, 1, 11, 2, 12, 3, 4];
    check(&v, || (0..5).into_par_iter().interleave(10..13));
    check(&v[..6], || (0..3).into_par_iter().interleave(10..13));

    let v = [0, 10, 1, 11, 2, 12, 13, 14];
    check(&v, || (0..3).into_par_iter().interleave(10..15));
}

#[test]
fn intersperse() {
    let v = [0, -1, 1, -1, 2, -1, 3, -1, 4];
    check(&v, || (0..5).into_par_iter().intersperse(-1));
}

#[test]
fn map() {
    let v: Vec<_> = (0..10).collect();
    check(&v, || v.par_iter().map(Clone::clone));
}

#[test]
fn map_with() {
    let v: Vec<_> = (0..10).collect();
    check(&v, || v.par_iter().map_with(vec![0], |_, &x| x));
}

#[test]
fn rev() {
    let v: Vec<_> = (0..10).rev().collect();
    check(&v, || (0..10).into_par_iter().rev());
}

#[test]
fn with_max_len() {
    let v: Vec<_> = (0..10).collect();
    check(&v, || (0..10).into_par_iter().with_max_len(1));
}

#[test]
fn with_min_len() {
    let v: Vec<_> = (0..10).collect();
    check(&v, || (0..10).into_par_iter().with_min_len(1));
}

#[test]
fn zip() {
    let v: Vec<_> = (0..10).zip(10..20).collect();
    check(&v, || (0..10).into_par_iter().zip(10..20));
    check(&v[..5], || (0..5).into_par_iter().zip(10..20));
    check(&v[..5], || (0..10).into_par_iter().zip(10..15));
}
