#![allow(dead_code, reason = "The crate is a WIP.")]

use std::{any::Any, error::Error, fmt::Display};

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
}

impl Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            FieldErrorReason::NoSuchType => write!(f, "No such type in this scope."),
        }
    }
}

impl Error for FieldError {}

struct FieldBuilder(Vec<Box<dyn Any>>);

impl FieldBuilder {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn add_field<T>(mut self, field: &T) -> Self
    where
        for<'a> T: 'a + Clone,
    {
        self.0.push(Box::new(field.clone()));

        self
    }

    // The first field whose `PartialEq` trait implementation compares equal
    // will be the one removed.
    fn rm_field<T>(&mut self, field: &T) -> Result<T, FieldError>
    where
        for<'a> T: 'a + PartialEq,
    {
        if let Some((idx, _)) = self.0.iter().enumerate().find(|&(_, elem)| {
            elem.is::<T>() && {
                elem.downcast_ref::<T>().expect(
                    "The prior check in the predicate conditional makes sure this is infallible.",
                ) == field
            }
        }) {
            Ok(*self.0.swap_remove(idx).downcast::<T>().expect(
                "The iterator chain that lead to this should make the operation infallible.",
            ))
        } else {
            Err(FieldError(FieldErrorReason::NoSuchType))
        }
    }

    fn consume_type<T>(&mut self) -> Result<FieldContainer<T>, FieldError>
    where
        for<'a> T: 'a,
    {
        let output: Vec<T> = self
            .0
            .extract_if(..self.0.len(), |elem| elem.is::<T>())
            .map(|elem| {
                *elem
                    .downcast::<T>()
                    .expect("The prior call in the iterator chain already made sure this was good.")
            })
            .collect();

        (!output.is_empty())
            .then(|| FieldContainer(output))
            .ok_or(FieldError(FieldErrorReason::NoSuchType))
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
    use super::*;

    #[test]
    fn it_works() {
        #[add_fields {
            T: Fields<String, 2>,
            T::Vertex: Fields<u32, 1>,
        }]
        fn planar_graph<T>(graph: &T)
        where
            T: GraphBackend + Fields<String, 2>,
            T::Vertex: Fields<u32, 1>,
        {
        }
    }
}
