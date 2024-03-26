use std::any::Any;

// from https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=f0cee315491dc3c3b6b3f467d6a3b072
// Provide a custom trait so that we can write a blanket implementation.
pub trait AsAny {
    fn as_any_ref(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn as_any_box(self: Box<Self>) -> Box<dyn Any>;

    fn type_name(&self) -> &'static str;
}

impl<T> AsAny for T
where
    T: Any,
{
    // This cast cannot be written in a default implementation so cannot be
    // moved to the original trait without implementing it for every type.
    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_box(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}
// helper here to avoid .as_any_ref() everywhere, with blanket implementation.
pub trait AsAnyHelper: AsAny {
    fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any_ref().downcast_ref()
    }
    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }
}
impl<T: AsAny + ?Sized> AsAnyHelper for T {}
