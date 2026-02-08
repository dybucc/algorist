use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::private::Sealed;

pub(crate) trait Field<T, const N: usize> {
    fn get(&self) -> &T;
    fn set(&mut self, other: &T);
}

pub(crate) trait Fields<T, const N: usize>
where
    Self: Sealed,
{
}

struct FieldBuilder(HashMap<TypeId, Vec<Box<dyn Any>>>);

// TODO: get the `TupleConstr` derive proc-macro fixed to work with the updated
// signature of `FieldBuilder`.
impl FieldBuilder {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn touch<T>(mut self) -> Self
    where
        for<'a> T: 'a + Default,
    {
        let ty_id = TypeId::of::<T>();
        self.0
            .entry(ty_id)
            .and_modify(|existing_fields| existing_fields.push(Box::new(T::default())))
            .or_insert_with(|| {
                // Need separate declaration because the inference algorithm
                // defaults to creating a `Box<T>` and not a `Box<dyn Any>`.
                let input: Box<dyn Any> = Box::new(T::default());

                vec![input]
            });

        self
    }

    // The first field whose `PartialEq` trait implementation compares equal
    // will be the one removed.
    fn rm<T>(&mut self) -> Option<()>
    where
        for<'a> T: 'a,
    {
        self.0.get_mut(&TypeId::of::<T>()).map(|fields| {
            fields.pop();
        })
    }

    fn own<T>(&mut self) -> Option<FieldContainer<T>>
    where
        for<'a> T: 'a,
    {
        self.0.remove(&TypeId::of::<T>()).map(|entry| {
            FieldContainer(
                entry
                    .into_iter()
                    .map(|elem| {
                        *elem.downcast::<T>().expect(
                            "`elem` should safely downcast to `T` because it's extracted from the \
                            `typeid` key of `T`.",
                        )
                    })
                    .collect(),
            )
        })
    }
}

struct FieldContainer<T>(Vec<T>);

impl<T> Deref for FieldContainer<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for FieldContainer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> AsRef<Vec<T>> for FieldContainer<T> {
    fn as_ref(&self) -> &Vec<T> {
        self
    }
}

impl<T> AsMut<Vec<T>> for FieldContainer<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        self
    }
}
