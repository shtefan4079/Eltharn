#![allow(dead_code)]
pub trait Cleaner<C, D> {
    fn register_for_cleaning(&self, slot: &CacheSlot<C, D>);
}
pub struct CacheSlot<C, D> {
    operation: Box<dyn Fn(&C) -> D>,
    context: Option<C>,
    value: Option<D>,
}
impl<C, D> CacheSlot<C, D>
where
    C: Clone + PartialEq,
    D: Clone,
{
    pub fn new<F>(operation: F) -> Self
    where
        F: Fn(&C) -> D + 'static,
    {
        Self {
            operation: Box::new(operation),
            context: None,
            value: None,
        }
    }
    pub fn compute<CL>(&mut self, context: &C, cleaner: &CL) -> D
    where
        CL: Cleaner<C, D>,
    {
        if let Some(ref current_ctx) = self.context {
            if current_ctx == context {
                if let Some(ref val) = self.value {
                    return val.clone();
                }
            }
        }
        let new_value = (self.operation)(context);
        self.value = Some(new_value.clone());
        self.context = Some(context.clone());
        cleaner.register_for_cleaning(self);
        new_value
    }
    pub fn clear(&mut self) {
        self.value = None;
        self.context = None;
    }
}
