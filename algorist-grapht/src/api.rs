use num_traits::AsPrimitive;

// TODO: consider further separating the API for graph backend implementors into
// a separate crate, such that utility methods may be exposed to users of that
// crate that should otherwise not be allowed to users of the graph generative
// routines.

pub(crate) trait GraphBackend
where
    Self: Sized,
{
    type Vertex;
    type Arc;

    type CreationResult;

    fn new<T>(n: T) -> Self::CreationResult
    where
        T: AsPrimitive<usize>;

    fn cmd_mut<T, U>(&mut self, cmd: T) -> U
    where
        T: CommandMut<U, Self>,
    {
        cmd.execute(self)
    }
    fn cmd<T, U>(&self, cmd: T) -> U
    where
        T: Command<U, Self>,
    {
        cmd.execute(self)
    }
}

pub(crate) trait Command<T, U>
where
    U: GraphBackend,
{
    fn execute(self, graph: &U) -> T;
}

pub(crate) trait CommandMut<T, U>
where
    U: GraphBackend,
{
    fn execute(self, graph: &mut U) -> T;
}
