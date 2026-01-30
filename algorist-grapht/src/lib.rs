#![allow(dead_code, reason = "The crate is a WIP.")]

use std::{
    any::{Any, TypeId},
    collections::{HashMap, hash_map::Entry},
    error::Error,
    fmt::Display,
};

#[derive(Debug)]
struct Arc<'a> {
    tip: &'a Vertex<'a>,
}

#[derive(Debug)]
struct Vertex<'a> {
    arcs: Vec<&'a Arc<'a>>,
}

#[derive(Debug)]
struct Graph<'a> {
    vertices: Vec<Vertex<'a>>,
    arcs: Vec<Arc<'a>>,
    n: usize,
    m: usize,
    id: Option<usize>,
}

impl<'a> GraphBackend for Graph<'a> {
    type Vertex = Vertex<'a>;
    type Arc = Arc<'a>;

    fn new(n: usize) -> Self {
        Graph {
            vertices: {
                let mut output = Vec::with_capacity(n);
                output.resize_with(n, || Vertex { arcs: Vec::new() });

                output
            },
            arcs: Vec::new(),
            n,
            m: 0,
            id: None,
        }
    }

    fn n(&self) -> usize {
        self.n
    }
    fn m(&self) -> usize {
        self.m
    }
}

#[derive(Debug)]
pub struct FieldError(pub FieldErrorReason);

#[non_exhaustive]
#[derive(Debug)]
pub enum FieldErrorReason {
    NoSuchType,
    EmptyBuilder,
}

impl Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            FieldErrorReason::NoSuchType => write!(f, "No such type in this scope."),
            FieldErrorReason::EmptyBuilder => write!(f, "No types within the `FieldBuilder`."),
        }
    }
}

impl Error for FieldError {}

struct FieldBuilder(HashMap<TypeId, Vec<Box<dyn Any>>>);

impl FieldBuilder {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn add_field<T>(mut self, field: T) -> Self
    where
        for<'a> T: 'a,
    {
        // Can't chain all entry API methods because both of their closures move
        // `field`.
        match self.0.entry(field.type_id()) {
            Entry::Vacant(_) => {
                self.0
                    .entry(field.type_id())
                    .and_modify(|existing_fields| existing_fields.push(Box::new(field)));
            }
            Entry::Occupied(_) => {
                self.0.entry(field.type_id()).or_insert_with(|| {
                    let mut output = Vec::new();
                    let input: Box<dyn Any> = Box::new(field);
                    output.push(input);

                    output
                });
            }
        }

        self
    }

    // The first field whose `PartialEq` trait implementation compares equal
    // will be the one removed.
    fn rm_field<T>(&mut self, field: &T) -> Option<T>
    where
        for<'a> T: 'a + PartialEq,
    {
        match self.0.get_mut(&field.type_id()) {
            Some(fields) => fields
                .iter()
                .position(|elem| elem.downcast_ref::<T>().unwrap() == field)
                .map(|idx| *fields.swap_remove(idx).downcast::<T>().unwrap()),
            None => None,
        }
    }

    fn consume_all(self) -> Option<Vec<Box<dyn impls::GenericFieldContainer>>> {
        (!self.0.is_empty()).then(|| {
            let mut output = Vec::new();

            self.0.into_values().for_each(|elem| {
                let input: Box<dyn impls::GenericFieldContainer> = Box::new(FieldContainer(elem));
                output.push(input);
            });

            output
        })
    }

    fn consume_type<T>(&mut self) -> Result<FieldContainer<T>, FieldError>
    where
        for<'a> T: 'a,
    {
        todo!()
    }
}

impl Default for FieldBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FieldContainer<T>(Vec<T>);

mod impls {
    use std::any::Any;

    use crate::FieldContainer;

    pub(crate) trait GenericFieldContainer
    where
        Self: Any,
    {
    }

    impl<T> GenericFieldContainer for FieldContainer<T> where T: Any {}
}

pub trait GenericFieldContainer<T> {
    fn extract(self) -> FieldContainer<T>;
}

impl<T> GenericFieldContainer<T> for Box<dyn impls::GenericFieldContainer>
where
    for<'a> T: 'a,
{
    fn extract(mut self) -> FieldContainer<T> {
        let src = unsafe { Box::from_raw(&mut self as *mut dyn Any) };
        *src.downcast().unwrap()
    }
}

pub trait Field<T, const N: usize> {
    fn get<'a>() -> &'a T;
    fn set(other: &T);
}

pub trait Fields<T, const N: usize> {}

pub trait GraphBackend {
    type Vertex;
    type Arc;

    fn new(n: usize) -> Self;

    fn n(&self) -> usize;
    fn m(&self) -> usize;
}

#[cfg(test)]
mod tests {
    use algorist_grapht_macros::add;

    use super::*;

    #[test]
    fn it_works() {
        // TODO: implement a macro that lets me access each field more
        // ergonomically inside of the function.
        #[add]
        fn planar_graph<T>(_: &T)
        where
            T: GraphBackend + Fields<String, 2>,
            T::Vertex: Fields<u32, 1>,
        {
            <T as Field<String, 0>>::get();
            <T::Vertex as Field<u32, 0>>::get();
        }
    }
}
