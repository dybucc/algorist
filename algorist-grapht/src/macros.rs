#[macro_export]
macro_rules! fields_of {
    ($num:expr; $ty:ty | $G:ty: $self:expr) => {{ <$G as $crate::api::FieldsExt<$ty, $num>>::chfield($self) }};
    ($num:expr; $ty:ty | $G:ty: $self:expr,f: $func:expr) => {{ <$G as $crate::api::FieldsExt<$ty, $num>>::chfield_with($self, $func) }};
    ($num:expr; $ty:ty | v in $G:ty: $self:expr) => {{
        <<$G as $crate::api::GraphBackend>::Vertex as $crate::api::FieldsExt<$ty, $num>>::chfield(
            $self,
        )
    }};
    ($num:expr; $ty:ty | v in $G:ty: $self:expr,f: $func:expr) => {{
        <<$G as $crate::api::GraphBackend>::Vertex as $crate::api::FieldsExt<$ty, $num>>::chfield_with( $self, $func, )
    }};
    ($num:expr; $ty:ty | a in $G:ty: $self:expr) => {{ <<G as $crate::api::GraphBackend>::Arc as $crate::api::FieldsExt<$ty, $num>>::chfield($self) }};
    ($num:expr; $ty:ty | a in $G:ty: $self:expr,f: $func:expr) => {{
        <<G as $crate::api::GraphBackend>::Arc as $crate::api::FieldsExt<$ty, $num>>::chfield_with(
            $self, $func,
        )
    }};
}
