#[cfg(any(doc, feature = "nightly"))]
pub(crate) mod nightly;
#[cfg(not(any(doc, feature = "nightly")))]
pub(crate) mod stable;

const INIT_ALLOC_CAPACITY: usize = 4;
