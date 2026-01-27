#![allow(dead_code, reason = "The crate is a WIP.")]

#[cfg(test)]
mod tests {
    use algorist_grapht_macros::TupleConstr;

    use std::mem::MaybeUninit;

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

        trait FieldElem {}

        impl<T> FieldElem for T {}

        #[derive(TupleConstr)]
        struct FieldBuilder {
            fields: Vec<Box<dyn FieldElem>>,
        }

        impl FieldBuilder {
            fn new() -> Self {
                Self { fields: Vec::new() }
            }

            // fn with_1(mut self, fields: (Box<dyn FieldElem>, Box<dyn FieldElem>)) -> Self {
            //     todo!()
            // }

            fn add_field(mut self, field: Box<dyn FieldElem>) -> Self {
                self.fields.push(field);

                self
            }
        }

        trait Field<T, const N: usize> {
            fn get(&self) -> &T;
            fn set(&mut self, other: &T);
        }

        trait GraphBackend {
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
