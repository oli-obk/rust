// Not in interpret to make sure we do not use private implementation details

use std::convert::{TryFrom, TryInto};

use rustc_hir::Mutability;
use rustc_middle::mir;
use rustc_middle::ty::ScalarInt;
use rustc_middle::{
    mir::interpret::{EvalToValTreeResult, GlobalId},
    ty::{self, Ty, TyCtxt},
};
use rustc_span::{source_map::DUMMY_SP, symbol::Symbol};
use rustc_target::abi::{LayoutOf, Size, VariantIdx};

use crate::interpret::{
    intern_const_alloc_recursive, ConstValue, InternKind, InterpCx, MPlaceTy, MemPlaceMeta, Scalar,
};

mod error;
mod eval_queries;
mod fn_queries;
mod machine;

pub use error::*;
pub use eval_queries::*;
pub use fn_queries::*;
pub use machine::*;

pub(crate) fn const_caller_location(
    tcx: TyCtxt<'tcx>,
    (file, line, col): (Symbol, u32, u32),
) -> ConstValue<'tcx> {
    trace!("const_caller_location: {}:{}:{}", file, line, col);
    let mut ecx = mk_eval_cx(tcx, DUMMY_SP, ty::ParamEnv::reveal_all(), false);

    let loc_place = ecx.alloc_caller_location(file, line, col);
    if intern_const_alloc_recursive(&mut ecx, InternKind::Constant, &loc_place).is_err() {
        bug!("intern_const_alloc_recursive should not error in this case")
    }
    ConstValue::Scalar(loc_place.ptr)
}

pub(crate) fn eval_to_valtree<'tcx>(
    tcx: TyCtxt<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    gid: GlobalId<'tcx>,
) -> EvalToValTreeResult<'tcx> {
    let raw = tcx.eval_to_allocation_raw(param_env.and(gid))?;
    let ecx = mk_eval_cx(
        tcx, DUMMY_SP, param_env,
        // It is absolutely crucial for soundness that
        // we do not read from static items or other mutable memory.
        false,
    );
    let place = ecx.raw_const_to_mplace(raw).unwrap();

    const_to_valtree(&ecx, &place)
}

fn branches<'tcx>(
    ecx: &CompileTimeEvalContext<'tcx, 'tcx>,
    n: usize,
    variant: Option<VariantIdx>,
    place: &MPlaceTy<'tcx>,
) -> EvalToValTreeResult<'tcx> {
    let place = match variant {
        Some(variant) => ecx.mplace_downcast(place, variant).unwrap(),
        None => *place,
    };
    let variant =
        variant.map(|variant| Ok(Some(ty::ValTree::Leaf(ScalarInt::from(variant.as_u32())))));
    let fields = (0..n).map(|i| {
        let field = ecx.mplace_field(&place, i).unwrap();
        const_to_valtree(ecx, &field)
    });
    // For enums, we preped their variant index before the variant's fields so we can figure out
    // the variant again when just seeing a valtree.
    let branches = variant.into_iter().chain(fields);
    let branches = branches.collect::<Result<Option<Vec<_>>, _>>()?;
    Ok(branches.map(|val| ty::ValTree::Branch(ecx.tcx.arena.alloc_from_iter(val))))
}

/// Convert an evaluated constant to a type level constant
fn const_to_valtree<'tcx>(
    ecx: &CompileTimeEvalContext<'tcx, 'tcx>,
    place: &MPlaceTy<'tcx>,
) -> EvalToValTreeResult<'tcx> {
    match place.layout.ty.kind() {
        ty::FnDef(..) => Ok(Some(ty::ValTree::zst())),
        ty::Bool | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Char => {
            let val = ecx.read_immediate(&place.into()).unwrap();
            let val = val.to_scalar().unwrap();
            Ok(Some(ty::ValTree::Leaf(val.assert_int())))
        }

        // Raw pointers are not allowed in type level constants, as we cannot properly test them for
        // equality at compile-time (see `ptr_guaranteed_eq`/`_ne`).
        // Technically we could allow function pointers (represented as `ty::Instance`), but this is not guaranteed to
        // agree with runtime equality tests.
        ty::FnPtr(_) | ty::RawPtr(_) => Ok(None),
        ty::Ref(..) => {
            let mplace = ecx.deref_operand(&place.into()).unwrap();
            if let Scalar::Ptr(ptr) = mplace.ptr {
                assert_eq!(
                    ecx.memory.get_raw(ptr.alloc_id).unwrap().mutability,
                    Mutability::Not,
                    "const_to_valtree cannot be used with mutable allocations as \
                    that could allow pattern matching to observe mutable statics",
                );
            }

            match mplace.meta {
                // We flatten references by encoding their dereferenced value directly.
                MemPlaceMeta::None => const_to_valtree(ecx, &mplace),
                MemPlaceMeta::Poison => bug!("poison metadata in `deref_const`: {:#?}", mplace),
                // In case of unsized types, figure out the real type behind.
                MemPlaceMeta::Meta(scalar) => {
                    let array = |elem_ty| {
                        let n = scalar.to_machine_usize(ecx).unwrap();
                        let mut mplace = mplace;
                        // Rewrite the layout to an array layout so the field accesses in `branches` work out.
                        mplace.layout = ecx.layout_of(ecx.tcx.mk_array(elem_ty, n)).unwrap();
                        branches(ecx, n.try_into().unwrap(), None, &mplace)
                    };
                    match mplace.layout.ty.kind() {
                        ty::Str => {
                            let n = scalar.to_machine_usize(ecx).unwrap();
                            if n > 0 {
                                let ptr = mplace.ptr.assert_ptr();
                                let s = ecx.memory.get_raw(ptr.alloc_id).unwrap().get_bytes(
                                    ecx,
                                    ptr,
                                    Size::from_bytes(n),
                                ).unwrap();
                                let s = std::str::from_utf8(s).unwrap();
                                let s = Symbol::intern(s);
                                Ok(Some(ty::ValTree::Str(s)))
                            } else {
                                Ok(Some(ty::ValTree::Str(Symbol::intern(""))))
                            }
                        },
                        // Slices are encoded as an array
                        ty::Slice(elem_ty) => array(elem_ty),
                        // No other unsized types are structural match.
                        _ => Ok(None),
                    }
                },
            }
        }

        // Trait objects are not allowed in type level constants, as we have no concept for
        // resolving their backing type, even if we can do that at const eval time. We may
        // hypothetically be able to allow `dyn StructuralEq` trait objects in the future,
        // but it is unclear if this is useful.
        ty::Dynamic(..) => Ok(None),

        ty::Slice(_) | ty::Str => {
            bug!("these are behind references and should have been handled there")
        }
        ty::Tuple(substs) => branches(ecx, substs.len(), None, place),
        ty::Array(_, len) => branches(ecx, usize::try_from(len.eval_usize(ecx.tcx.tcx, ecx.param_env)).unwrap(), None, place),

        ty::Adt(def, _) if place.layout.ty.is_structural_eq_shallow(ecx.tcx.tcx) => {
            if def.variants.is_empty() {
                bug!("uninhabited types should have errored and never gotten converted to valtree")
            }

            let variant = ecx.read_discriminant(&place.into()).unwrap().1;

            branches(ecx, def.variants[variant].fields.len(), def.is_enum().then_some(variant), place)
        }

        ty::Never => bug!("CTFE computed a value of never type without erroring"),

        // Types that do not derive PartialEq are not converted to valtree
        ty::Adt(..)
        | ty::Error(_)
        | ty::Foreign(..)
        // These are probably unreachable
        | ty::Infer(ty::FreshIntTy(_))
        | ty::Infer(ty::FreshFloatTy(_))
        | ty::Projection(..)
        // Should have errored out during CTFE due to being too polymorphic
        | ty::Param(_)
        // These are probably unreachable
        | ty::Bound(..)
        | ty::Placeholder(..)
        // FIXME(oli-obk): we could look behind opaque types
        | ty::Opaque(..)
        // This is probably unreachable
        | ty::Infer(_)
        // FIXME(oli-obk): we can probably encode closures just like structs
        | ty::Closure(..)
        | ty::Generator(..)
        | ty::GeneratorWitness(..) => Ok(None),
    }
}

/// This function uses `unwrap` copiously, because an already validated constant
/// must have valid fields and can thus never fail outside of compiler bugs. However, it is
/// invoked from the pretty printer, where it can receive enums with no variants and e.g.
/// `read_discriminant` needs to be able to handle that.
pub(crate) fn destructure_const<'tcx>(
    tcx: TyCtxt<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    val: ConstValue<'tcx>,
    ty: Ty<'tcx>,
) -> mir::DestructuredConst<'tcx> {
    trace!("destructure_const: {:?}", val);
    let ecx = mk_eval_cx(tcx, DUMMY_SP, param_env, false);
    let op = ecx.const_val_to_op(val, ty, None).unwrap();

    // We go to `usize` as we cannot allocate anything bigger anyway.
    let (field_count, variant, down) = match op.layout.ty.kind() {
        ty::Array(_, len) => (usize::try_from(len.eval_usize(tcx, param_env)).unwrap(), None, op),
        ty::Adt(def, _) if def.variants.is_empty() => {
            return mir::DestructuredConst { variant: None, fields: &[] };
        }
        ty::Adt(def, _) => {
            let variant = ecx.read_discriminant(&op).unwrap().1;
            let down = ecx.operand_downcast(&op, variant).unwrap();
            (def.variants[variant].fields.len(), Some(variant), down)
        }
        ty::Tuple(substs) => (substs.len(), None, op),
        _ => bug!("cannot destructure constant {:?}", val),
    };

    let fields_iter = (0..field_count).map(|i| {
        let field_op = ecx.operand_field(&down, i).unwrap();
        let val = op_to_const(&ecx, &field_op);
        (val, field_op.layout.ty)
    });
    let fields = tcx.arena.alloc_from_iter(fields_iter);

    mir::DestructuredConst { variant, fields }
}

pub(crate) fn deref_const<'tcx>(
    tcx: TyCtxt<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    val: ConstValue<'tcx>,
    ty: Ty<'tcx>,
) -> (ConstValue<'tcx>, Ty<'tcx>) {
    trace!("deref_const: {:?}", val);
    let ecx = mk_eval_cx(tcx, DUMMY_SP, param_env, false);
    let op = ecx.const_val_to_op(val, ty, None).unwrap();
    let mplace = ecx.deref_operand(&op).unwrap();
    if let Scalar::Ptr(ptr) = mplace.ptr {
        assert_eq!(
            ecx.memory.get_raw(ptr.alloc_id).unwrap().mutability,
            Mutability::Not,
            "deref_const cannot be used with mutable allocations as \
            that could allow pattern matching to observe mutable statics",
        );
    }

    let ty = match mplace.meta {
        MemPlaceMeta::None => mplace.layout.ty,
        MemPlaceMeta::Poison => bug!("poison metadata in `deref_const`: {:#?}", mplace),
        // In case of unsized types, figure out the real type behind.
        MemPlaceMeta::Meta(scalar) => match mplace.layout.ty.kind() {
            ty::Str => bug!("there's no sized equivalent of a `str`"),
            ty::Slice(elem_ty) => tcx.mk_array(elem_ty, scalar.to_machine_usize(&tcx).unwrap()),
            _ => bug!(
                "type {} should not have metadata, but had {:?}",
                mplace.layout.ty,
                mplace.meta
            ),
        },
    };

    (op_to_const(&ecx, &mplace.into()), ty)
}
