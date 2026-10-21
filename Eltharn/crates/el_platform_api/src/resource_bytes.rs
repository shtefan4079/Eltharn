use std::{
    ops::{Deref, Range},
    sync::Arc,
};
#[derive(Clone)]
pub struct ResourceBytes {
    owner: Arc<dyn AsRef<[u8]> + Send + Sync>,
    range: Range<usize>,
}
impl ResourceBytes {
    pub fn from_owner(owner: impl AsRef<[u8]> + Send + Sync + 'static) -> Self {
        let len = owner.as_ref().len();
        Self {
            owner: Arc::new(owner),
            range: 0..len,
        }
    }
    pub fn from_shared(bytes: Arc<Vec<u8>>) -> Self {
        struct Shared(Arc<Vec<u8>>);
        impl AsRef<[u8]> for Shared {
            fn as_ref(&self) -> &[u8] {
                self.0.as_slice()
            }
        }
        Self::from_owner(Shared(bytes))
    }
    pub fn slice(&self, range: Range<usize>) -> Self {
        assert!(range.start <= range.end && range.end <= self.len());
        Self {
            owner: self.owner.clone(),
            range: self.range.start + range.start..self.range.start + range.end,
        }
    }
}
impl Deref for ResourceBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.owner.as_ref().as_ref()[self.range.clone()]
    }
}
impl AsRef<[u8]> for ResourceBytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}
impl std::fmt::Debug for ResourceBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceBytes")
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}
