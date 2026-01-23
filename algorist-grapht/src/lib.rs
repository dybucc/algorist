#![allow(dead_code, reason = "The crate is a WIP.")]

#[cfg(test)]
mod tests {
    use std::mem::MaybeUninit;

    #[test]
    fn it_works() {
        #[derive(Debug)]
        struct Arc {
            tip: Vertex,
        }

        #[derive(Debug)]
        struct Vertex {
            arcs: Vec<Arc>,
        }

        #[derive(Debug)]
        struct AdjacencyList(Vec<Vertex>);

        #[derive(Debug)]
        struct Graph {
            vertices: Vec<AdjacencyList>,
            n: usize,
            m: usize,
            id: usize,
        }

        let mut graph: MaybeUninit<Graph> = MaybeUninit::uninit();
        let graph_handle = graph.as_mut_ptr();

        unsafe {
            (&raw mut (*graph_handle).n).write(2);
            (&raw mut (*graph_handle).m).write(3);
        }

        let graph = unsafe { graph.assume_init() };
        println!("{:#?}", graph.n);
    }
}
