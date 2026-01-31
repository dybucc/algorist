#![allow(dead_code, reason = "The crate is a WIP.")]

use algorist_grapht_macros::add;
use std::{
    any::{Any, TypeId},
    collections::{HashMap, hash_map::Entry},
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
    // TODO: see whether this deserves its own API; The generative routines
    // don't seem to mind it much, but something could be implemented around
    // existing functionality from the original GraphBase. See GB_GRAPH, Sec.
    // 26, 27.
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

    #[doc(alias = "len", alias = "verts")]
    fn n(&self) -> usize {
        self.n
    }
    #[doc(alias = "len", alias = "arcs")]
    fn m(&self) -> usize {
        self.m
    }

    fn add_arc(&mut self) {}
}

struct FieldBuilder(HashMap<TypeId, Vec<Box<dyn Any>>>);

// TODO: get the `TupleConstr` derive proc-macro fixed to work with the updated
// signature of `FieldBuilder`.
impl FieldBuilder {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn add_field<T>(mut self, field: T) -> Self
    where
        for<'a> T: 'a + PartialEq,
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
                    let input: Box<dyn Any> = Box::new(field);

                    vec![input]
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

    fn consume_fields_of<T>(&mut self) -> Option<FieldContainer<T>>
    where
        for<'a> T: 'a,
    {
        self.0.remove(&TypeId::of::<T>()).map(|entry| {
            FieldContainer(
                entry
                    .into_iter()
                    .map(|elem| *elem.downcast::<T>().unwrap())
                    .collect(),
            )
        })
    }
}

impl Default for FieldBuilder {
    fn default() -> Self {
        Self::new()
    }
}

struct FieldContainer<T>(Vec<T>);

pub trait Field<T, const N: usize> {
    fn get<'a>() -> &'a T;
    fn set(other: &T);
}

trait Fields<T, const N: usize> {}

pub trait GraphBackend {
    type Vertex;
    type Arc;

    fn new(n: usize) -> Self;

    fn n(&self) -> usize;
    fn m(&self) -> usize;

    fn add_arc(&mut self);
}

impl<T> Fields<String, 2> for T where T: Field<String, 0> + Field<String, 1> {}

#[expect(private_bounds)]
#[cfg_attr(not(doc), add)]
pub fn planar_graph<T>(_: &T)
where
    T: GraphBackend + Fields<String, 2>,
    T::Vertex: Fields<u32, 1>,
{
    <T as Field<String, 0>>::get();
    <T::Vertex as Field<u32, 0>>::get();
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
