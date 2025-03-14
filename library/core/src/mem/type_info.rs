//! MVP for exposing compile-time information about types in a
//! runtime or const-eval processable way.

/// Compile-time type information.
#[derive(Debug)]
#[non_exhaustive]
#[lang = "type_info"]
#[unstable(feature = "type_info", issue = "none")]
pub struct Type {
    /// Per-type information
    pub kind: TypeKind,
    /// Size of the type
    pub size: Option<usize>,
}

/// A reference to [crate::any::TypeId]. Cannot be inspected during CTFE,
/// but can be used to query  types
#[derive(Debug, Copy, Clone)]
#[lang = "type_info_id"]
#[unstable(feature = "type_info", issue = "none")]
pub struct TypeId(&'static crate::any::TypeId);

/// Compute the type information of a concrete type.
/// It can only be called at compile time, the backends do
/// not implement it.
#[cfg_attr(not(bootstrap), rustc_intrinsic)]
const fn type_of(id: TypeId) -> &'static Type;

/// Compute the type information of a concrete type.
/// It can only be called at compile time, the backends do
/// not implement it.
#[cfg_attr(not(bootstrap), rustc_intrinsic)]
const fn type_id_of<T>() -> TypeId;

impl TypeId {
    /// Compute the type information of a concrete type.
    /// It can only be called at compile time.
    #[unstable(feature = "type_info", issue = "none")]
    #[rustc_const_unstable(feature = "type_info", issue = "none")]
    pub const fn info(self) -> &'static Type {
        fn todo(_: TypeId) -> &'static Type {
            todo!()
        }
        crate::intrinsics::const_eval_select((self,), type_of, todo)
    }

    /// Get the runtime type id
    #[unstable(feature = "type_info", issue = "none")]
    pub fn type_id(self) -> crate::any::TypeId {
        *self.0
    }

    /// Returns the `TypeId` of the generic type parameter.
    #[unstable(feature = "type_info", issue = "none")]
    #[rustc_const_unstable(feature = "type_info", issue = "none")]
    pub const fn of<T>() -> Self {
        const { type_id_of::<T>() }
    }
}

impl Type {
    /// Returns the type information of the generic type parameter.
    #[unstable(feature = "type_info", issue = "none")]
    #[rustc_const_unstable(feature = "type_info", issue = "none")]
    pub const fn of<T>() -> &'static Self {
        const { type_id_of::<T>().info() }
    }
}

/// Compile-time type information.
#[derive(Debug)]
#[non_exhaustive]
#[unstable(feature = "type_info", issue = "none")]
pub enum TypeKind {
    /// Tuples.
    Tuple(Tuple),
    /// Primitives
    Leaf,
    /// TODO
    Unimplemented,
}

/// Compile-time type information about tuples.
#[derive(Debug)]
#[non_exhaustive]
#[unstable(feature = "type_info", issue = "none")]
pub struct Tuple {
    /// All fields of a tuple.
    pub fields: &'static [Field],
}

/// Compile-time type information about fields of tuples, structs and enum variants.
#[derive(Debug)]
#[non_exhaustive]
#[unstable(feature = "type_info", issue = "none")]
pub struct Field {
    /// The field's type.
    pub ty: TypeId,
    /// Offset in bytes from the parent type
    pub offset: usize,
}
