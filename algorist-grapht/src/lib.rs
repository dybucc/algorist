#![allow(dead_code, reason = "The crate is a WIP.")]

#[cfg(test)]
mod tests {
    use std::{any::Any, mem::MaybeUninit};

    #[test]
    fn it_works() {
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

        pub struct FieldBuilder(Vec<Box<dyn Any>>);

        impl FieldBuilder {
            pub fn new() -> Self {
                Self(Vec::new())
            }

            pub fn add_field<T>(mut self, field: &T) -> Self
            where
                for<'a> T: 'a + Any + Clone,
            {
                self.0.push(Box::new(field.clone()));

                self
            }

            pub fn rm_field<T>(mut self, field: &T) -> Result<T, Self>
            where
                for<'a> T: 'a + Any + PartialEq,
            {
                if let Some((idx, _)) = self.0.iter().enumerate().find(|&(_, elem)| {
                    elem.is::<T>() && {
                        elem.downcast_ref::<T>().expect(
                            "The prior check in the predicate conditional makes sure this is \
                            infallible.",
                        ) == field
                    }
                }) {
                    Ok(*self.0.swap_remove(idx).downcast::<T>().expect(
                        "The iterator chain that lead to this should make the operation \
                        infallible.",
                    ))
                } else {
                    Err(self)
                }
            }

            pub fn consume_type<T>(&mut self) -> FieldContainer<T>
            where
                for<'a> T: 'a,
            {
                FieldContainer(
                    self.0
                        .extract_if(..self.0.len(), |elem| elem.is::<T>())
                        .map(|elem| {
                            *elem.downcast::<T>().expect(
                            "The prior call in the iterator chain already made sure this was good.",
                        )
                        })
                        .collect(),
                )
            }
        }

        pub struct FieldContainer<T>(Vec<T>);

        pub trait Field<T, const N: usize> {
            fn get(&self) -> &T;
            fn set(&mut self, other: &T);
        }

        pub trait GraphBackend {
            type Vertex;
            type Arc;

            fn new(n: usize) -> Self;

            fn n(&self) -> usize;
            fn m(&self) -> usize;
        }

        let mut graph: MaybeUninit<Graph> = MaybeUninit::uninit();
        let graph_handle = graph.as_mut_ptr();

        unsafe {
            (&raw mut (*graph_handle).n).write(2);
            (&raw mut (*graph_handle).m).write(3);
        }
    }
}
