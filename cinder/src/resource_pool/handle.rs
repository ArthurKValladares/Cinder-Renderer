use std::{cmp::Ordering, fmt, hash, marker::PhantomData, num::NonZeroU32, ops};

pub struct Handle<T> {
    bits: NonZeroU32,
    marker: std::marker::PhantomData<T>,
}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle").field("bits", &self.bits).finish()
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            bits: self.bits,
            marker: self.marker,
        }
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bits == other.bits
    }
}

impl<T> Eq for Handle<T> {}

impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.bits.partial_cmp(&other.bits)
    }
}

impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bits.cmp(&other.bits)
    }
}

impl<T> hash::Hash for Handle<T> {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.bits.hash(hasher)
    }
}
