#![allow(dead_code, reason = "The crate is a WIP.")]

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::{Deref, DerefMut},
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

impl<'a, 'b> GraphBackend<'b> for Graph<'a>
where
    'b: 'a,
{
    type Vertex = Vertex<'a>;
    type Arc = Arc<'a>;
    type Error = GraphError;

    const ARC_ALLOCS: usize = 102;
    const EXTRA_N: usize = 4;

    fn new(n: usize) -> Self {
        Graph {
            vertices: {
                let mut output = Vec::with_capacity(n + Self::EXTRA_N);
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

    fn new_arc<T>(&mut self, u: &'b T, v: &'b T) -> Result<(), Self::Error>
    where
        T: AsRef<Self::Vertex>,
    {
        self.arcs.reserve(Self::ARC_ALLOCS);
        self.vertices
            .last_mut()
            .map(|elem| {
                self.arcs.push(Arc { tip: v.as_ref() });
                elem.arcs.push(self.arcs.last().unwrap());
            })
            .ok_or(GraphError(GraphErrorReason::NoVerticesInGraph))
    }
    fn new_edge<T>(&mut self, u: &T, v: &T)
    where
        T: AsRef<Self::Vertex>,
    {
    }
}

struct GraphError(GraphErrorReason);

enum GraphErrorReason {
    NoVerticesInGraph,
}

struct FieldBuilder(HashMap<TypeId, Vec<Box<dyn Any>>>);

// TODO: get the `TupleConstr` derive proc-macro fixed to work with the updated
// signature of `FieldBuilder`.
impl FieldBuilder {
    fn new() -> Self {
        Self(HashMap::new())
    }

    #[doc(alias = "insert", alias = "add")]
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
                // defaults to creating a `Box` of `T` and not a trait object.
                let input: Box<dyn Any> = Box::new(T::default());

                vec![input]
            });

        self
    }

    // The first field whose `PartialEq` trait implementation compares equal
    // will be the one removed.
    #[doc(alias = "remove", alias = "del", alias = "delete")]
    fn rm<T>(&mut self) -> Option<()>
    where
        for<'a> T: 'a,
    {
        self.0.get_mut(&TypeId::of::<T>()).map(|fields| {
            fields.pop();
        })
    }

    #[doc(alias = "consume")]
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

impl Default for FieldBuilder {
    fn default() -> Self {
        Self::new()
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
        self.deref()
    }
}

impl<T> AsMut<Vec<T>> for FieldContainer<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        self.deref_mut()
    }
}

trait Field<T, const N: usize> {
    fn get<'a>() -> &'a T;
    fn set(other: &T);
}

trait Fields<T, const N: usize> {}

trait GraphBackend<'a> {
    type Vertex;
    type Arc;
    type Error;

    const ARC_ALLOCS: usize;
    const EXTRA_N: usize;

    fn new(n: usize) -> Self;

    fn n(&self) -> usize;
    fn m(&self) -> usize;

    fn new_arc<T>(&mut self, u: &'a T, v: &'a T) -> Result<(), Self::Error>
    where
        T: AsRef<Self::Vertex>;
    fn new_edge<T>(&mut self, u: &T, v: &T)
    where
        T: AsRef<Self::Vertex>;
}

#[cfg(test)]
mod tests {
    use algorist_grapht_macros::add;

    use super::*;

    #[test]
    fn it_works() {
        // TODO: implement a macro that lets me access each field more
        // ergonomically inside of the function.
        #[cfg_attr(not(doc), add)]
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
