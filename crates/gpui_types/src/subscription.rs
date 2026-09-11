use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    fmt::Debug,
    rc::Rc,
};

/// A collection of callbacks grouped by the identity of the thing they observe.
///
/// This type is an implementation detail of GPUI's application contexts. It is public only so
/// those contexts can use the backend-neutral subscription implementation.
#[doc(hidden)]
pub struct SubscriberSet<EmitterKey, Callback>(
    Rc<RefCell<SubscriberSetState<EmitterKey, Callback>>>,
);

impl<EmitterKey, Callback> Clone for SubscriberSet<EmitterKey, Callback> {
    fn clone(&self) -> Self {
        SubscriberSet(self.0.clone())
    }
}

struct SubscriberSetState<EmitterKey, Callback> {
    subscribers: BTreeMap<EmitterKey, Option<BTreeMap<usize, Subscriber<Callback>>>>,
    next_subscriber_id: usize,
}

struct Subscriber<Callback> {
    active: Rc<Cell<bool>>,
    dropped: Rc<Cell<bool>>,
    callback: Callback,
}

impl<EmitterKey, Callback> SubscriberSet<EmitterKey, Callback>
where
    EmitterKey: 'static + Ord + Clone + Debug,
    Callback: 'static,
{
    /// Creates an empty set of subscribers.
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(SubscriberSetState {
            subscribers: Default::default(),
            next_subscriber_id: 0,
        })))
    }

    /// Inserts a new [`Subscription`] for the given `emitter_key`. By default, subscriptions
    /// are inert, meaning that they won't be listed when calling [`SubscriberSet::remove`] or
    /// [`SubscriberSet::retain`]. This method returns a tuple of a [`Subscription`] and an
    /// activation function that can be used to activate the subscription.
    pub fn insert(
        &self,
        emitter_key: EmitterKey,
        callback: Callback,
    ) -> (Subscription, impl FnOnce() + use<EmitterKey, Callback>) {
        let active = Rc::new(Cell::new(false));
        let dropped = Rc::new(Cell::new(false));
        let mut lock = self.0.borrow_mut();
        let subscriber_id = lock.next_subscriber_id;
        lock.next_subscriber_id += 1;
        lock.subscribers
            .entry(emitter_key.clone())
            .or_default()
            .get_or_insert_with(Default::default)
            .insert(
                subscriber_id,
                Subscriber {
                    active: active.clone(),
                    dropped: dropped.clone(),
                    callback,
                },
            );
        let this = self.0.clone();

        let subscription = Subscription {
            unsubscribe: Some(Box::new(move || {
                dropped.set(true);

                let mut lock = this.borrow_mut();
                let Some(subscribers) = lock.subscribers.get_mut(&emitter_key) else {
                    return;
                };

                if let Some(subscribers) = subscribers {
                    subscribers.remove(&subscriber_id);
                    if subscribers.is_empty() {
                        lock.subscribers.remove(&emitter_key);
                    }
                }
            })),
        };
        (subscription, move || active.set(true))
    }

    /// Removes all subscribers for an emitter and returns the active callbacks.
    pub fn remove(
        &self,
        emitter: &EmitterKey,
    ) -> impl IntoIterator<Item = Callback> + use<EmitterKey, Callback> {
        let subscribers = self.0.borrow_mut().subscribers.remove(emitter);
        subscribers
            .unwrap_or_default()
            .map(|subscribers| subscribers.into_values())
            .into_iter()
            .flatten()
            .filter_map(|subscriber| {
                if subscriber.active.get() {
                    Some(subscriber.callback)
                } else {
                    None
                }
            })
    }

    /// Calls the given callback for each active subscriber to an emitter.
    /// If the callback returns false, the subscriber is removed.
    pub fn retain<F>(&self, emitter: &EmitterKey, mut callback: F)
    where
        F: FnMut(&mut Callback) -> bool,
    {
        let Some(mut subscribers) = self
            .0
            .borrow_mut()
            .subscribers
            .get_mut(emitter)
            .and_then(|subscribers| subscribers.take())
        else {
            return;
        };

        subscribers.retain(|_, subscriber| {
            if !subscriber.active.get() {
                return true;
            }
            if subscriber.dropped.get() {
                return false;
            }
            let keep = callback(&mut subscriber.callback);
            keep && !subscriber.dropped.get()
        });
        let mut lock = self.0.borrow_mut();

        // Add any new subscribers that were added while invoking the callback.
        if let Some(Some(new_subscribers)) = lock.subscribers.remove(emitter) {
            subscribers.extend(new_subscribers);
        }

        if !subscribers.is_empty() {
            lock.subscribers.insert(emitter.clone(), Some(subscribers));
        }
    }
}

/// A handle to a subscription created by GPUI. When dropped, the subscription is cancelled and
/// the callback will no longer be invoked.
#[must_use]
pub struct Subscription {
    unsubscribe: Option<Box<dyn FnOnce() + 'static>>,
}

impl Subscription {
    /// Creates a new subscription with a callback that gets invoked when this subscription is
    /// dropped.
    pub fn new(unsubscribe: impl 'static + FnOnce()) -> Self {
        Self {
            unsubscribe: Some(Box::new(unsubscribe)),
        }
    }

    /// Detaches the subscription from this handle. The callback will continue to be invoked
    /// until the entities it has been subscribed to are dropped.
    pub fn detach(mut self) {
        self.unsubscribe.take();
    }

    /// Joins two subscriptions into a single subscription. Detaching the joined subscription
    /// detaches both interior subscriptions.
    pub fn join(mut subscription_a: Self, mut subscription_b: Self) -> Self {
        let a_unsubscribe = subscription_a.unsubscribe.take();
        let b_unsubscribe = subscription_b.unsubscribe.take();
        Self {
            unsubscribe: Some(Box::new(move || {
                if let Some(unsubscribe) = a_unsubscribe {
                    unsubscribe();
                }
                if let Some(unsubscribe) = b_unsubscribe {
                    unsubscribe();
                }
            })),
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(unsubscribe) = self.unsubscribe.take() {
            unsubscribe();
        }
    }
}

impl std::fmt::Debug for Subscription {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Subscription").finish()
    }
}

impl crate::SubscriptionHandle for Subscription {
    fn detach(self) {
        Subscription::detach(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{SubscriberSet, Subscription};
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn subscription_runs_unsubscribe_on_drop_and_not_on_detach() {
        let dropped = Rc::new(Cell::new(false));
        let subscription = Subscription::new({
            let dropped = dropped.clone();
            move || dropped.set(true)
        });
        drop(subscription);
        assert!(dropped.get());

        let detached = Rc::new(Cell::new(false));
        Subscription::new({
            let detached = detached.clone();
            move || detached.set(true)
        })
        .detach();
        assert!(!detached.get());
    }

    #[test]
    fn joined_subscription_runs_both_unsubscribe_callbacks() {
        let first = Rc::new(Cell::new(false));
        let second = Rc::new(Cell::new(false));
        let joined = Subscription::join(
            Subscription::new({
                let first = first.clone();
                move || first.set(true)
            }),
            Subscription::new({
                let second = second.clone();
                move || second.set(true)
            }),
        );

        drop(joined);
        assert!(first.get());
        assert!(second.get());
    }

    #[test]
    fn subscriber_set_only_returns_active_callbacks() {
        let subscribers = SubscriberSet::<u8, usize>::new();
        let (inactive, _) = subscribers.insert(0, 1);
        let (active, activate) = subscribers.insert(0, 2);
        activate();

        let values: Vec<_> = subscribers.remove(&0).into_iter().collect();
        assert_eq!(values, vec![2]);

        drop(inactive);
        drop(active);
    }

    #[test]
    fn subscriber_set_retain_removes_callbacks_that_return_false() {
        let subscribers = SubscriberSet::<u8, usize>::new();
        let (first, activate_first) = subscribers.insert(0, 1);
        activate_first();
        let (second, activate_second) = subscribers.insert(0, 2);
        activate_second();

        subscribers.retain(&0, |value| {
            *value += 1;
            *value < 3
        });

        let values: Vec<_> = subscribers.remove(&0).into_iter().collect();
        assert_eq!(values, vec![2]);

        drop(first);
        drop(second);
    }
}
