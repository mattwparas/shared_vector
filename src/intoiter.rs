use std::{
    mem,
    ptr::{self, NonNull},
};

use crate::{RawVector, Vector};

use crate::alloc::{Allocator, Global};

pub struct IntoIter<T, A: Allocator = Global> {
    _buf: RawVector<T>, // we don't actually care about this. Just need it to live.
    iter: RawValIter<T>,
    pub(crate) allocator: A,
}

impl<T, A: Allocator> Iterator for IntoIter<T, A> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

impl<T, A: Allocator> Drop for IntoIter<T, A> {
    fn drop(&mut self) {
        // drop any remaining elements
        for _ in &mut *self {}

        unsafe {
            self._buf.deallocate_no_drop(&self.allocator);
        }
    }
}

impl<T, A: Allocator> IntoIterator for Vector<T, A> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        let (iter, buf) = unsafe { (RawValIter::new(&self), ptr::read(&self.raw)) };

        mem::forget(self);

        IntoIter {
            iter,
            _buf: buf,
            allocator: Global,
        }
    }
}

struct RawValIter<T> {
    start: *const T,
    end: *const T,
}

impl<T> RawValIter<T> {
    unsafe fn new(slice: &[T]) -> Self {
        RawValIter {
            start: slice.as_ptr(),
            end: if mem::size_of::<T>() == 0 {
                ((slice.as_ptr() as usize) + slice.len()) as *const _
            } else if slice.len() == 0 {
                slice.as_ptr()
            } else {
                slice.as_ptr().add(slice.len())
            },
        }
    }
}

impl<T> Iterator for RawValIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                if mem::size_of::<T>() == 0 {
                    self.start = (self.start as usize + 1) as *const _;
                    Some(ptr::read(NonNull::<T>::dangling().as_ptr()))
                } else {
                    let old_ptr = self.start;
                    self.start = self.start.offset(1);
                    Some(ptr::read(old_ptr))
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elem_size = mem::size_of::<T>();
        let len =
            (self.end as usize - self.start as usize) / if elem_size == 0 { 1 } else { elem_size };
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for RawValIter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                if mem::size_of::<T>() == 0 {
                    self.end = (self.end as usize - 1) as *const _;
                    Some(ptr::read(NonNull::<T>::dangling().as_ptr()))
                } else {
                    self.end = self.end.offset(-1);
                    Some(ptr::read(self.end))
                }
            }
        }
    }
}

impl<T> ExactSizeIterator for RawValIter<T> {}

#[cfg(test)]
mod tests {
    #[test]
    fn into_iter_test() {
        struct Foo {
            value: Box<i32>,
        }

        impl Foo {
            pub fn new(value: i32) -> Self {
                Self {
                    value: Box::new(value),
                }
            }
        }

        impl Drop for Foo {
            fn drop(&mut self) {}
        }

        let mut vector = crate::Vector::new();

        for i in 0..=100 {
            vector.push(Foo::new(i));
        }

        let resulting = vector.into_iter().collect::<Vec<_>>();

        let sum = resulting.into_iter().map(|x| *x.value).sum::<i32>();

        assert_eq!(sum, 5050)
    }
}
