//! This module contains the parallel iterator types for results
//! (`Result<T, E>`). You will rarely need to interact with it directly
//! unless you have need to name one of the iterator types.

use iter::*;
use iter::internal::*;
use std::sync::Mutex;

use option;

/// Parallel iterator over a result
#[derive(Debug, Clone)]
pub struct IntoIter<T: Send> {
    inner: option::IntoIter<T>,
}

impl<T: Send, E> IntoParallelIterator for Result<T, E> {
    type Item = T;
    type Iter = IntoIter<T>;

    fn into_par_iter(self) -> Self::Iter {
        IntoIter { inner: self.ok().into_par_iter() }
    }
}

delegate_indexed_iterator!{
    IntoIter<T> => T,
    impl<T: Send>
}


/// Parallel iterator over an immutable reference to a result
#[derive(Debug)]
pub struct Iter<'a, T: Sync + 'a> {
    inner: option::IntoIter<&'a T>,
}

impl<'a, T: Sync> Clone for Iter<'a, T> {
    fn clone(&self) -> Self {
        Iter { inner: self.inner.clone() }
    }
}

impl<'a, T: Sync, E> IntoParallelIterator for &'a Result<T, E> {
    type Item = &'a T;
    type Iter = Iter<'a, T>;

    fn into_par_iter(self) -> Self::Iter {
        Iter { inner: self.as_ref().ok().into_par_iter() }
    }
}

delegate_indexed_iterator!{
    Iter<'a, T> => &'a T,
    impl<'a, T: Sync + 'a>
}


/// Parallel iterator over a mutable reference to a result
#[derive(Debug)]
pub struct IterMut<'a, T: Send + 'a> {
    inner: option::IntoIter<&'a mut T>,
}

impl<'a, T: Send, E> IntoParallelIterator for &'a mut Result<T, E> {
    type Item = &'a mut T;
    type Iter = IterMut<'a, T>;

    fn into_par_iter(self) -> Self::Iter {
        IterMut { inner: self.as_mut().ok().into_par_iter() }
    }
}

delegate_indexed_iterator!{
    IterMut<'a, T> => &'a mut T,
    impl<'a, T: Send + 'a>
}


/// Collect an arbitrary `Result`-wrapped collection.
///
/// If any item is `Err`, then all previous `Ok` items collected are
/// discarded, and it returns that error.  If there are multiple errors, the
/// one returned is not deterministic.
impl<'a, C, T, E> FromParallelIterator<Result<T, E>> for Result<C, E>
    where C: FromParallelIterator<T>,
          T: Send,
          E: Send
{
    fn from_par_iter<I>(par_iter: I) -> Self
        where I: IntoParallelIterator<Item = Result<T, E>>
    {
        let saved_error = Mutex::new(None);
        let collection = par_iter
            .into_par_iter()
            .map(|item| match item {
                     Ok(item) => Some(item),
                     Err(error) => {
                         if let Ok(mut guard) = saved_error.lock() {
                             *guard = Some(error);
                         }
                         None
                     }
                 })
            .while_some()
            .collect();

        match saved_error.into_inner().unwrap() {
            Some(error) => Err(error),
            None => Ok(collection),
        }
    }
}
