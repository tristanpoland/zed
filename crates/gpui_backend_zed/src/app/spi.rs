use crate::{AnyEntity, AnyWeakEntity, Entity, EntityId, Subscription, Task, WeakEntity};

/// The base SPI implemented by an application context provider.
pub trait AppContextSpi: EntityContextSpi + GlobalContextSpi + TaskContextSpi {}

/// The entity operations supplied by an application context provider.
pub trait EntityContextSpi {}

/// The global-state operations supplied by an application context provider.
pub trait GlobalContextSpi {}

/// The task operations supplied by an application context provider.
pub trait TaskContextSpi {}

/// The window operations supplied by an application context provider.
pub trait WindowContextSpi: AppContextSpi {}

impl<T: ?Sized> AppContextSpi for T where T: EntityContextSpi + GlobalContextSpi + TaskContextSpi {}
impl<T: ?Sized> EntityContextSpi for T {}
impl<T: ?Sized> GlobalContextSpi for T {}
impl<T: ?Sized> TaskContextSpi for T {}
impl<T: ?Sized> WindowContextSpi for T where T: AppContextSpi {}

/// The common contract exposed by every entity handle.
pub trait EntityHandle {
    /// Returns the identifier of the referenced entity.
    fn entity_id(&self) -> EntityId;
}

/// The contract exposed by a strong, typed entity handle.
pub trait StrongEntityHandle<T>: EntityHandle {
    /// The corresponding weak handle type.
    type Weak: WeakEntityHandle<T>;

    /// Downgrades this handle without changing the entity's lifetime.
    fn downgrade(&self) -> Self::Weak;
}

/// The contract exposed by a weak, typed entity handle.
pub trait WeakEntityHandle<T>: EntityHandle {
    /// The corresponding strong handle type.
    type Strong: StrongEntityHandle<T>;

    /// Attempts to upgrade this handle.
    fn upgrade(&self) -> Option<Self::Strong>;
}

impl<T: 'static> EntityHandle for Entity<T> {
    fn entity_id(&self) -> EntityId {
        Entity::entity_id(self)
    }
}

impl<T: 'static> StrongEntityHandle<T> for Entity<T> {
    type Weak = WeakEntity<T>;

    fn downgrade(&self) -> Self::Weak {
        Entity::downgrade(self)
    }
}

impl<T> EntityHandle for WeakEntity<T> {
    fn entity_id(&self) -> EntityId {
        std::ops::Deref::deref(self).entity_id()
    }
}

impl<T: 'static> WeakEntityHandle<T> for WeakEntity<T> {
    type Strong = Entity<T>;

    fn upgrade(&self) -> Option<Self::Strong> {
        WeakEntity::upgrade(self)
    }
}

impl EntityHandle for AnyEntity {
    fn entity_id(&self) -> EntityId {
        AnyEntity::entity_id(self)
    }
}

impl EntityHandle for AnyWeakEntity {
    fn entity_id(&self) -> EntityId {
        AnyWeakEntity::entity_id(self)
    }
}

/// The cancellation contract exposed by a scheduled task.
pub trait TaskHandle<T> {
    /// Detaches the task so it runs independently of this handle.
    fn detach(self);
}

impl<T> TaskHandle<T> for Task<T> {
    fn detach(self) {
        Task::detach(self)
    }
}

/// The cancellation contract exposed by a subscription.
pub trait SubscriptionHandle {
    /// Detaches the subscription while preserving its callback.
    fn detach(self);
}

impl SubscriptionHandle for Subscription {
    fn detach(self) {
        Subscription::detach(self)
    }
}
