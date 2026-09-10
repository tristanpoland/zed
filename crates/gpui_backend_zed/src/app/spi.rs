pub use gpui_types::{
    AppContextSpi, EntityHandle, StrongEntityHandle, SubscriptionHandle, WeakEntityHandle,
};

use crate::{AnyEntity, AnyWeakEntity, App, Entity, EntityId, Subscription, Task, WeakEntity};

/// The cancellation contract exposed by a scheduled task.
pub trait TaskHandle<T> {
    /// Detaches the task so it runs independently of this handle.
    fn detach(self);
}

impl AppContextSpi for App {
    fn entity_exists(&self, entity_id: EntityId) -> bool {
        self.entities.contains(entity_id)
    }
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

impl<T> TaskHandle<T> for Task<T> {
    fn detach(self) {
        Task::detach(self)
    }
}

impl SubscriptionHandle for Subscription {
    fn detach(self) {
        Subscription::detach(self)
    }
}

const _: () = {
    const fn assert_app_context<T: AppContextSpi>() {}
    const fn assert_entity_handle<T: EntityHandle>() {}
    const fn assert_strong_entity_handle<T: StrongEntityHandle<()>>() {}
    const fn assert_weak_entity_handle<T: WeakEntityHandle<()>>() {}

    assert_app_context::<App>();
    assert_entity_handle::<Entity<()>>();
    assert_entity_handle::<WeakEntity<()>>();
    assert_entity_handle::<AnyEntity>();
    assert_entity_handle::<AnyWeakEntity>();
    assert_strong_entity_handle::<Entity<()>>();
    assert_weak_entity_handle::<WeakEntity<()>>();
};
