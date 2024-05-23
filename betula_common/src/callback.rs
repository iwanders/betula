use parking_lot::RwLock;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::{Arc, Weak};

type CallbackFun<T> = Weak<Arc<dyn Fn(T) -> ()>>;

/// The ticket is returned after registration, keep this around to ensure the registration stays active.
pub struct Ticket<T> {
    _registration: Arc<Arc<dyn Fn(T) -> ()>>,
}
impl<T> std::fmt::Debug for Ticket<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Ticket<{}>", std::any::type_name::<T>())
    }
}

// #[derive(Default)]
pub struct Callbacks<T: Send + 'static + Clone> {
    callbacks: RwLock<Vec<CallbackFun<T>>>,
}
impl<T: Send + 'static + Clone> Default for Callbacks<T> {
    fn default() -> Self {
        Self {
            callbacks: RwLock::new(vec![]),
        }
    }
}

impl<T: Send + 'static + Clone> Callbacks<T> {
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
    pub fn register<F: Fn(T) -> () + 'static>(&self, f: F) -> Ticket<T> {
        self.register_arc(Arc::new(f))
    }

    /// Register a callback with this callbacks object.
    pub fn register_arc(&self, f: Arc<dyn Fn(T) -> ()>) -> Ticket<T> {
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
        let to_call: Vec<Arc<Arc<dyn Fn(T) -> ()>>> =
            ptrs.iter().filter_map(|v| v.upgrade()).collect();

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
pub struct CallbacksBlackboard<T: Send + 'static + Clone> {
    callbacks: Arc<Option<Callbacks<T>>>,
    count: usize,
}

impl<T: Send + 'static + Clone> CallbacksBlackboard<T> {
    pub fn new() -> Self {
        Self {
            callbacks: Arc::new(Some(Default::default())),
            count: 0,
        }
    }
    pub fn callbacks(&self) -> Option<&Callbacks<T>> {
        self.callbacks.as_ref().as_ref()
    }
}

impl<T: Send + 'static + Clone> std::fmt::Debug for CallbacksBlackboard<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "CB<{}>({})",
            std::any::type_name::<T>(),
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

impl<T: Send + 'static + Clone> Serialize for CallbacksBlackboard<T> {
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

impl<'de, T: Send + 'static + Clone> Deserialize<'de> for CallbacksBlackboard<T> {
    fn deserialize<D>(deserializer: D) -> Result<CallbacksBlackboard<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let t = CallbacksDummy::deserialize(deserializer)?;
        Ok(CallbacksBlackboard {
            count: t.count,
            callbacks: Arc::new(Default::default()),
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
