use parking_lot::RwLock;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::{Arc, Weak};

pub trait CallbackValueRequirements: Send + 'static + Clone + Sync {}
impl<T: Send + 'static + Clone + Sync> CallbackValueRequirements for T {}

type CallbackFun<T> = Weak<Arc<dyn Fn(T) + Send + Sync>>;

/// The ticket is returned after registration, keep this around to ensure the registration stays active.
pub struct Ticket<T> {
    _registration: Arc<Arc<dyn Fn(T) + Send + Sync>>,
}
impl<T> std::fmt::Debug for Ticket<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Ticket<{}>", std::any::type_name::<T>())
    }
}

#[derive(Clone)]
pub struct Callbacks<T: CallbackValueRequirements> {
    callbacks: Arc<RwLock<Vec<CallbackFun<T>>>>,
}
impl<T: CallbackValueRequirements> std::fmt::Debug for Callbacks<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Callbacks<{}>({})", std::any::type_name::<T>(), {
            let z = self.callbacks.read();
            z.len()
        })
    }
}
impl<T: CallbackValueRequirements> std::cmp::PartialEq for Callbacks<T> {
    fn eq(&self, other: &Callbacks<T>) -> bool {
        Arc::as_ptr(&self.callbacks) == Arc::as_ptr(&other.callbacks)
    }
}

impl<T: CallbackValueRequirements> Default for Callbacks<T> {
    fn default() -> Self {
        Self {
            callbacks: Arc::new(RwLock::new(vec![])),
        }
    }
}

impl<T: CallbackValueRequirements> Callbacks<T> {
    /// Create a new callbacks object.
    pub fn new() -> Self {
        Callbacks {
            callbacks: Default::default(),
        }
    }

    /// The number of callbacks.
    pub fn callback_count(&self) -> usize {
        let locked = self.callbacks.read();
        locked.len()
    }

    /// Register a callback with this callbacks object.
    pub fn register<F: Fn(T) + 'static + Send + Sync>(&self, f: F) -> Ticket<T> {
        self.register_arc(Arc::new(f))
    }

    /// Register a callback with this callbacks object.
    pub fn register_arc(&self, f: Arc<dyn Fn(T) + Send + Sync>) -> Ticket<T> {
        // Wrap the function in a new arc, of which we keep the weak pointer.
        // We hand the strong pointer back in the ticket, that way if the ticket goes out of
        // scope, we know the registration has become invalid.
        let registration = Arc::new(f);
        let our_weak = Arc::downgrade(&registration);

        let mut locked = self.callbacks.write();
        locked.push(our_weak);

        Ticket {
            _registration: registration,
        }
    }

    pub fn call(&self, data: T) {
        // let mut to_call: Vec<Arc<Arc<dyn Fn(T) -> ()>>> = vec![];
        // Under the lock, check what we can make strong and drop everything we can't.
        let ptrs = {
            let locked = self.callbacks.read();
            locked.clone()
        };
        let orig = ptrs.len();
        let to_call: Vec<Arc<Arc<_>>> = ptrs.iter().filter_map(|v| v.upgrade()).collect();

        // Length changed, we actaully need to prune registrations.
        if orig != to_call.len() {
            // Something may have added while we did our upgrades, so lets just repeat it under
            // the lock.
            let mut locked = self.callbacks.write();
            *locked = locked
                .iter()
                .filter_map(|v| v.upgrade().map(|z| Arc::downgrade(&z)))
                .collect();
        }
        // Finaly, we can call the strong poinetrs.
        for f in to_call {
            (f)(data.clone())
        }
    }
}

#[derive(Default, Clone)]
pub struct CallbacksBlackboard<T: CallbackValueRequirements> {
    callbacks: Option<Callbacks<T>>,
    count: usize,
}
impl<T: CallbackValueRequirements> std::cmp::PartialEq for CallbacksBlackboard<T> {
    fn eq(&self, other: &CallbacksBlackboard<T>) -> bool {
        if let Some(ours) = self.callbacks.as_ref() {
            if let Some(other) = other.callbacks.as_ref() {
                return other == ours;
            }
        }
        false
    }
}

impl<T: CallbackValueRequirements> CallbacksBlackboard<T> {
    pub fn new() -> Self {
        Self {
            callbacks: Some(Default::default()),
            count: 0,
        }
    }
    pub fn callbacks(&self) -> Option<&Callbacks<T>> {
        self.callbacks.as_ref()
    }
}

impl<T: CallbackValueRequirements> std::fmt::Debug for CallbacksBlackboard<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let tname = std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap_or(std::any::type_name::<T>());
        write!(
            fmt,
            "CB<{}>({})",
            tname,
            self.callbacks
                .as_ref()
                .as_ref()
                .map(|v| v.callback_count())
                .unwrap_or(self.count)
        )
    }
}

#[derive(Deserialize, Serialize)]
struct CallbacksDummy {
    count: usize,
}

impl<T: CallbackValueRequirements> Serialize for CallbacksBlackboard<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let count = {
            self.callbacks
                .as_ref()
                .as_ref()
                .map(|v| v.callback_count())
                .unwrap_or(0)
            // 0
        };
        let v = CallbacksDummy { count };
        CallbacksDummy::serialize(&v, serializer)
    }
}

impl<'de, T: CallbackValueRequirements> Deserialize<'de> for CallbacksBlackboard<T> {
    fn deserialize<D>(deserializer: D) -> Result<CallbacksBlackboard<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let t = CallbacksDummy::deserialize(deserializer)?;
        Ok(CallbacksBlackboard {
            count: t.count,
            callbacks: Default::default(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_callbacks() -> Result<(), Box<dyn std::error::Error>> {
        let mut z: Callbacks<i32> = Default::default();
        let mut r1 = Some(z.register(|v| println!("1: {v:?}")));
        let mut r2 = Some(z.register_arc(Arc::new(|v| println!("2: {v:?}"))));

        z.call(1);
        r1.take();
        z.call(2);
        r2.take();
        println!("Nothing left to call");
        z.call(3);

        // test as blackboard value.
        let zbb = CallbacksBlackboard::<i32>::new();
        let mut r1 = Some(zbb.callbacks().unwrap().register(|v| println!("1: {v:?}")));
        println!("r1: {r1:?}");
        println!("zbb: {zbb:?}");
        let zbb_json = serde_json::to_string(&zbb)?;
        let back: CallbacksBlackboard<i32> = serde_json::from_str(&zbb_json)?;
        println!("back: {back:?}");
        Ok(())
    }
}
