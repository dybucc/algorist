#[macro_export]
macro_rules! fields_of {
    ($ty:ty; $num:expr => $G:ty: $self:expr) => {{ <$G as $crate::api::FieldsExt<$ty, $num>>::chfield($self) }};
    ($ty:ty; $num:expr => $G:ty: $self:expr,f: $func:expr) => {{ <$G as $crate::api::FieldsExt<$ty, $num>>::chfield_with($self, $func) }};
    ($ty:ty; $num:expr => v in $G:ty: $self:expr) => {{
        <<$G as $crate::api::GraphBackend>::Vertex as $crate::api::FieldsExt<$ty, $num>>::chfield(
            $self,
        )
    }};
    ($ty:ty; $num:expr => v in $G:ty: $self:expr,f: $func:expr) => {{
        <<$G as $crate::api::GraphBackend>::Vertex as $crate::api::FieldsExt<$ty, $num>>::chfield_with( $self, $func, )
    }};
    ($ty:ty; $num:expr => a in $G:ty: $self:expr) => {{ <<G as $crate::api::GraphBackend>::Arc as $crate::api::FieldsExt<$ty, $num>>::chfield($self) }};
    ($ty:ty; $num:expr => a in $G:ty: $self:expr,f: $func:expr) => {{
        <<G as $crate::api::GraphBackend>::Arc as $crate::api::FieldsExt<$ty, $num>>::chfield_with(
            $self, $func,
        )
    }};
}
