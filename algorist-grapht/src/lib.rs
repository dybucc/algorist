#![allow(dead_code, reason = "The crate is a WIP.")]
#[cfg(test)]
mod tests {
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
        impl Graph<'_> {
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
        }
        trait GraphId {
            fn id(&self) -> usize;
        }
        trait GraphBackend
        where
            Self: GraphId,
        {
            fn new() -> Self;

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
