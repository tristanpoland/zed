use super::{EntityHandle, StrongEntityHandle, WeakEntityHandle};
use std::{
    any::{TypeId, type_name},
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::{Rc, Weak},
    sync::atomic::{AtomicU64, Ordering},
};

use slotmap::KeyData;

slotmap::new_key_type! {
    /// A unique identifier for an entity across an application.
    pub struct EntityId;
}

impl From<u64> for EntityId {
    fn from(value: u64) -> Self {
        Self(KeyData::from_ffi(value))
    }
}

impl EntityId {
    /// Converts this entity id to a [`std::num::NonZeroU64`].
    pub fn as_non_zero_u64(self) -> std::num::NonZeroU64 {
        std::num::NonZeroU64::new(self.0.as_ffi()).unwrap()
    }

    /// Converts this entity id to a `u64`.
    pub fn as_u64(self) -> u64 {
        self.0.as_ffi()
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.as_u64())
    }
}

/// The backend capability used by shared entity handles to retain and release
/// strong ownership and to upgrade weak ownership.
pub trait EntityHandleRuntime: 'static {
    /// Retains one strong reference to an entity.
    fn retain(&self, entity_id: EntityId);

    /// Releases one strong reference to an entity.
    fn release(&self, entity_id: EntityId);

    /// Returns whether a strong reference can currently be acquired.
    fn is_upgradable(&self, entity_id: EntityId) -> bool;

    /// Atomically upgrades one weak reference when the entity is still live.
    fn upgrade(&self, entity_id: EntityId) -> bool {
        if !self.is_upgradable(entity_id) {
            return false;
        }
        self.retain(entity_id);
        true
    }
}

/// A type-erased strong entity handle.
///
/// The handle stores its identity and a backend capability object. The
/// capability owns no entity state itself; it adapts the backend's existing
/// reference-count and lifetime implementation to this shared type layer.
pub struct AnyEntity {
    entity_id: EntityId,
    entity_type: TypeId,
    runtime: Rc<dyn EntityHandleRuntime>,
}

impl AnyEntity {
    /// Creates a handle for an existing live entity and retains it.
    pub fn from_parts(
        entity_id: EntityId,
        entity_type: TypeId,
        runtime: Rc<dyn EntityHandleRuntime>,
    ) -> Option<Self> {
        if !runtime.is_upgradable(entity_id) {
            return None;
        }
        runtime.retain(entity_id);
        Some(Self {
            entity_id,
            entity_type,
            runtime,
        })
    }

    /// Creates the first strong handle for a slot whose backend reference
    /// count has already been reserved.
    #[doc(hidden)]
    pub fn from_reserved_parts(
        entity_id: EntityId,
        entity_type: TypeId,
        runtime: Rc<dyn EntityHandleRuntime>,
    ) -> Self {
        Self {
            entity_id,
            entity_type,
            runtime,
        }
    }

    /// Returns the id associated with this entity.
    #[inline]
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    /// Returns the [`TypeId`] associated with this entity.
    #[inline]
    pub fn entity_type(&self) -> TypeId {
        self.entity_type
    }

    /// Converts this entity handle into a weak variant.
    pub fn downgrade(&self) -> AnyWeakEntity {
        AnyWeakEntity {
            entity_id: self.entity_id,
            entity_type: self.entity_type,
            runtime: Some(Rc::downgrade(&self.runtime)),
        }
    }

    /// Converts this entity handle into a strongly typed handle.
    pub fn downcast<T: 'static>(self) -> Result<Entity<T>, AnyEntity> {
        if TypeId::of::<T>() == self.entity_type {
            Ok(Entity {
                any_entity: self,
                entity_type: PhantomData,
            })
        } else {
            Err(self)
        }
    }
}

impl Clone for AnyEntity {
    fn clone(&self) -> Self {
        self.runtime.retain(self.entity_id);
        Self {
            entity_id: self.entity_id,
            entity_type: self.entity_type,
            runtime: self.runtime.clone(),
        }
    }
}

impl Drop for AnyEntity {
    fn drop(&mut self) {
        self.runtime.release(self.entity_id);
    }
}

impl<T> From<Entity<T>> for AnyEntity {
    #[inline]
    fn from(entity: Entity<T>) -> Self {
        entity.any_entity
    }
}

impl EntityHandle for AnyEntity {
    fn entity_id(&self) -> EntityId {
        self.entity_id
    }
}

impl Hash for AnyEntity {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity_id.hash(state);
    }
}

impl PartialEq for AnyEntity {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.entity_id == other.entity_id
    }
}

impl Eq for AnyEntity {}

impl Ord for AnyEntity {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.entity_id.cmp(&other.entity_id)
    }
}

impl PartialOrd for AnyEntity {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for AnyEntity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnyEntity")
            .field("entity_id", &self.entity_id.as_u64())
            .field("entity_type", &self.entity_type)
            .finish()
    }
}

/// A strong, well-typed reference to backend-managed state.
pub struct Entity<T> {
    any_entity: AnyEntity,
    entity_type: PhantomData<fn(T) -> T>,
}

impl<T> Entity<T> {
    /// Creates the first strong handle for a reserved entity slot.
    #[doc(hidden)]
    pub fn from_reserved_parts(entity_id: EntityId, runtime: Rc<dyn EntityHandleRuntime>) -> Self
    where
        T: 'static,
    {
        Self {
            any_entity: AnyEntity::from_reserved_parts(entity_id, TypeId::of::<T>(), runtime),
            entity_type: PhantomData,
        }
    }

    /// Creates a strong handle for an existing live entity.
    pub fn from_parts(entity_id: EntityId, runtime: Rc<dyn EntityHandleRuntime>) -> Option<Self>
    where
        T: 'static,
    {
        Some(Self {
            any_entity: AnyEntity::from_parts(entity_id, TypeId::of::<T>(), runtime)?,
            entity_type: PhantomData,
        })
    }

    /// Returns the entity id.
    #[inline]
    pub fn entity_id(&self) -> EntityId {
        self.any_entity.entity_id()
    }

    /// Returns the type id recorded for this handle.
    #[inline]
    pub fn entity_type(&self) -> TypeId {
        self.any_entity.entity_type()
    }

    /// Downgrades this entity pointer to a non-retaining weak pointer.
    #[inline]
    pub fn downgrade(&self) -> WeakEntity<T> {
        WeakEntity {
            any_entity: self.any_entity.downgrade(),
            entity_type: PhantomData,
        }
    }

    /// Converts this into a dynamically typed entity.
    #[inline]
    pub fn into_any(self) -> AnyEntity {
        self.any_entity
    }
}

impl<T> EntityHandle for Entity<T> {
    fn entity_id(&self) -> EntityId {
        self.entity_id()
    }
}

impl<T: 'static> StrongEntityHandle<T> for Entity<T> {
    type Weak = WeakEntity<T>;

    fn downgrade(&self) -> Self::Weak {
        self.downgrade()
    }
}

impl<T> Clone for Entity<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            any_entity: self.any_entity.clone(),
            entity_type: self.entity_type,
        }
    }
}

impl<T> Deref for Entity<T> {
    type Target = AnyEntity;

    fn deref(&self) -> &Self::Target {
        &self.any_entity
    }
}

impl<T> DerefMut for Entity<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.any_entity
    }
}

impl<T> fmt::Debug for Entity<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Entity")
            .field("entity_id", &self.entity_id())
            .field("entity_type", &type_name::<T>())
            .finish()
    }
}

impl<T> Hash for Entity<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.any_entity.hash(state);
    }
}

impl<T> PartialEq for Entity<T> {
    fn eq(&self, other: &Self) -> bool {
        self.any_entity == other.any_entity
    }
}

impl<T> Eq for Entity<T> {}

impl<T> PartialEq<WeakEntity<T>> for Entity<T> {
    fn eq(&self, other: &WeakEntity<T>) -> bool {
        self.entity_id() == other.entity_id()
    }
}

impl<T> Ord for Entity<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.entity_id().cmp(&other.entity_id())
    }
}

impl<T> PartialOrd for Entity<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A type-erased weak entity reference.
#[derive(Clone)]
pub struct AnyWeakEntity {
    entity_id: EntityId,
    entity_type: TypeId,
    runtime: Option<Weak<dyn EntityHandleRuntime>>,
}

impl AnyWeakEntity {
    /// Returns the entity id.
    #[inline]
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    /// Returns the type id recorded for this handle.
    #[inline]
    pub fn entity_type(&self) -> TypeId {
        self.entity_type
    }

    /// Returns whether this weak handle can be upgraded.
    pub fn is_upgradable(&self) -> bool {
        self.runtime
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|runtime| runtime.is_upgradable(self.entity_id))
    }

    /// Attempts to upgrade this weak entity reference.
    pub fn upgrade(&self) -> Option<AnyEntity> {
        let runtime = self.runtime.as_ref()?.upgrade()?;
        if !runtime.upgrade(self.entity_id) {
            return None;
        }
        Some(AnyEntity {
            entity_id: self.entity_id,
            entity_type: self.entity_type,
            runtime,
        })
    }

    /// Creates an invalid weak handle for use as an optional sentinel.
    #[doc(hidden)]
    pub fn new_invalid() -> Self {
        static UNIQUE_NON_CONFLICTING_ID_GENERATOR: AtomicU64 = AtomicU64::new(u64::MAX);
        let entity_id = UNIQUE_NON_CONFLICTING_ID_GENERATOR.fetch_sub(1, Ordering::SeqCst);

        Self {
            entity_id: entity_id.into(),
            entity_type: TypeId::of::<()>(),
            runtime: None,
        }
    }
}

impl EntityHandle for AnyWeakEntity {
    fn entity_id(&self) -> EntityId {
        self.entity_id
    }
}

impl fmt::Debug for AnyWeakEntity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnyWeakEntity")
            .field("entity_id", &self.entity_id)
            .field("entity_type", &self.entity_type)
            .finish()
    }
}

impl<T> From<WeakEntity<T>> for AnyWeakEntity {
    fn from(entity: WeakEntity<T>) -> Self {
        entity.any_entity
    }
}

impl Hash for AnyWeakEntity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity_id.hash(state);
    }
}

impl PartialEq for AnyWeakEntity {
    fn eq(&self, other: &Self) -> bool {
        self.entity_id == other.entity_id
    }
}

impl Eq for AnyWeakEntity {}

impl Ord for AnyWeakEntity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.entity_id.cmp(&other.entity_id)
    }
}

impl PartialOrd for AnyWeakEntity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A typed weak entity reference.
pub struct WeakEntity<T> {
    any_entity: AnyWeakEntity,
    entity_type: PhantomData<fn(T) -> T>,
}

impl<T> Clone for WeakEntity<T> {
    fn clone(&self) -> Self {
        Self {
            any_entity: self.any_entity.clone(),
            entity_type: self.entity_type,
        }
    }
}

impl<T> WeakEntity<T> {
    /// Returns the entity id.
    #[inline]
    pub fn entity_id(&self) -> EntityId {
        self.any_entity.entity_id()
    }

    /// Returns the type id recorded for this handle.
    #[inline]
    pub fn entity_type(&self) -> TypeId {
        self.any_entity.entity_type()
    }

    /// Returns whether this weak handle can be upgraded.
    pub fn is_upgradable(&self) -> bool {
        self.any_entity.is_upgradable()
    }

    /// Attempts to upgrade this weak entity reference.
    pub fn upgrade(&self) -> Option<Entity<T>>
    where
        T: 'static,
    {
        Some(Entity {
            any_entity: self.any_entity.upgrade()?,
            entity_type: PhantomData,
        })
    }

    /// Creates an invalid weak handle for use as an optional sentinel.
    #[doc(hidden)]
    pub fn new_invalid() -> Self {
        Self {
            any_entity: AnyWeakEntity::new_invalid(),
            entity_type: PhantomData,
        }
    }
}

impl<T> EntityHandle for WeakEntity<T> {
    fn entity_id(&self) -> EntityId {
        self.entity_id()
    }
}

impl<T: 'static> WeakEntityHandle<T> for WeakEntity<T> {
    type Strong = Entity<T>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.upgrade()
    }
}

impl<T> Deref for WeakEntity<T> {
    type Target = AnyWeakEntity;

    fn deref(&self) -> &Self::Target {
        &self.any_entity
    }
}

impl<T> DerefMut for WeakEntity<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.any_entity
    }
}

impl<T> fmt::Debug for WeakEntity<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WeakEntity")
            .field("entity_id", &self.entity_id())
            .field("entity_type", &type_name::<T>())
            .finish()
    }
}

impl<T> Hash for WeakEntity<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.any_entity.hash(state);
    }
}

impl<T> PartialEq for WeakEntity<T> {
    fn eq(&self, other: &Self) -> bool {
        self.any_entity == other.any_entity
    }
}

impl<T> Eq for WeakEntity<T> {}

impl<T> PartialEq<Entity<T>> for WeakEntity<T> {
    fn eq(&self, other: &Entity<T>) -> bool {
        self.entity_id() == other.entity_id()
    }
}

impl<T> Ord for WeakEntity<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.entity_id().cmp(&other.entity_id())
    }
}

impl<T> PartialOrd for WeakEntity<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::{AnyEntity, Entity, EntityHandleRuntime, EntityId};
    use std::{
        cell::Cell,
        rc::Rc,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct TestRuntime {
        strong_count: AtomicUsize,
        retains: Cell<usize>,
        releases: Cell<usize>,
    }

    impl EntityHandleRuntime for TestRuntime {
        fn retain(&self, _: EntityId) {
            self.strong_count.fetch_add(1, Ordering::SeqCst);
            self.retains.set(self.retains.get() + 1);
        }

        fn release(&self, _: EntityId) {
            self.strong_count.fetch_sub(1, Ordering::SeqCst);
            self.releases.set(self.releases.get() + 1);
        }

        fn is_upgradable(&self, _: EntityId) -> bool {
            self.strong_count.load(Ordering::SeqCst) > 0
        }
    }

    #[test]
    fn shared_handles_preserve_strong_and_weak_ownership() {
        let runtime = Rc::new(TestRuntime {
            strong_count: AtomicUsize::new(1),
            retains: Cell::new(0),
            releases: Cell::new(0),
        });
        let runtime_capability: Rc<dyn EntityHandleRuntime> = runtime.clone();
        let entity_id = EntityId::from(1);
        let entity = Entity::<u32>::from_reserved_parts(entity_id, runtime_capability);
        let weak = entity.downgrade();
        let any = entity.clone().into_any();

        assert!(weak.is_upgradable());
        assert_eq!(any.entity_id(), entity_id);
        assert_eq!(runtime.retains.get(), 1);

        drop(entity);
        drop(any);
        assert!(weak.upgrade().is_none());
        assert_eq!(runtime.releases.get(), 2);
    }

    #[test]
    fn type_erased_handles_downcast_by_type_id() {
        let runtime = Rc::new(TestRuntime {
            strong_count: AtomicUsize::new(1),
            retains: Cell::new(0),
            releases: Cell::new(0),
        });
        let runtime_capability: Rc<dyn EntityHandleRuntime> = runtime.clone();
        let any = AnyEntity::from_reserved_parts(
            EntityId::from(2),
            std::any::TypeId::of::<u32>(),
            runtime_capability,
        );
        assert!(any.downcast::<u32>().is_ok());
    }
}
