use rustc_abi::{Size, VariantIdx};
use rustc_hir::LangItem;
use rustc_middle::mir::interpret::{CtfeProvenance, Pointer, Scalar};
use rustc_middle::span_bug;
use rustc_middle::ty::layout::LayoutOf as _;
use rustc_middle::ty::{self, ScalarInt, Ty};
use rustc_span::sym;

use super::{InterpCx, InterpResult, MPlaceTy, MemoryKind, interp_ok};
use crate::const_eval::CompileTimeMachine;
use crate::interpret::Machine as _;

impl<'tcx> InterpCx<'tcx, CompileTimeMachine<'tcx>> {
    pub(crate) fn build_type_info(
        &mut self,
        ty: Ty<'tcx>,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, CtfeProvenance>> {
        let layout = self.layout_of(ty)?;
        let ty_struct = self.tcx.require_lang_item(LangItem::Type, Some(self.tcx.span));
        let ty_struct = self.tcx.type_of(ty_struct).instantiate_identity();
        let ty_struct_layout = self.layout_of(ty_struct)?;
        let ty_struct = ty_struct.ty_adt_def().unwrap().non_enum_variant();
        let mplace = self.allocate(ty_struct_layout, MemoryKind::Stack)?;
        for (idx, field) in ty_struct.fields.iter_enumerated() {
            let mplace = self.project_field(&mplace, idx.as_usize())?;
            match field.name {
                sym::kind => {
                    let variant_index = match ty.kind() {
                        ty::Tuple(fields) => {
                            // TODO: properly obtain the variant indices
                            let variant = VariantIdx::from_usize(0);
                            // `Tuple` variant
                            let mplace = self.project_downcast(&mplace, variant)?;
                            // `Tuple` struct
                            let mplace = self.project_field(&mplace, 0)?;
                            assert_eq!(
                                1,
                                mplace
                                    .layout
                                    .ty
                                    .ty_adt_def()
                                    .unwrap()
                                    .non_enum_variant()
                                    .fields
                                    .len()
                            );
                            // `fields` field
                            let mplace = self.project_field(&mplace, 0)?;
                            let field_type = mplace
                                .layout
                                .ty
                                .builtin_deref(false)
                                .unwrap()
                                .sequence_element_type(self.tcx.tcx);
                            let fields_layout = self.layout_of(Ty::new_array(
                                self.tcx.tcx,
                                field_type,
                                fields.len() as u64,
                            ))?;
                            let fields_place = self.allocate(fields_layout, MemoryKind::Stack)?;
                            let mut fields_places = self.project_array_fields(&fields_place)?;
                            while let Some((i, place)) = fields_places.next(self)? {
                                let field_ty = fields[i as usize];
                                for (idx, field_ty_field) in place
                                    .layout
                                    .ty
                                    .ty_adt_def()
                                    .unwrap()
                                    .non_enum_variant()
                                    .fields
                                    .iter_enumerated()
                                {
                                    let field_place = self.project_field(&place, idx.as_usize())?;
                                    match field_ty_field.name {
                                        sym::ty => {
                                            let alloc_id =
                                                self.tcx.reserve_and_set_type_id_alloc(field_ty);
                                            let ptr =
                                                CompileTimeMachine::adjust_alloc_root_pointer(
                                                    self,
                                                    Pointer::new(alloc_id.into(), Size::ZERO),
                                                    None,
                                                )?;
                                            self.write_scalar(
                                                Scalar::from_pointer(ptr, &self.tcx),
                                                &field_place,
                                            )?;
                                        }
                                        sym::offset => {
                                            let offset = layout.fields.offset(i as usize);
                                            self.write_scalar(
                                                ScalarInt::try_from_target_usize(
                                                    offset.bytes(),
                                                    self.tcx.tcx,
                                                )
                                                .unwrap(),
                                                &field_place,
                                            )?;
                                        }
                                        other => span_bug!(
                                            self.tcx.def_span(field_ty_field.did),
                                            "unimplemented field {other}"
                                        ),
                                    }
                                }
                            }

                            let fields_place =
                                fields_place.map_provenance(CtfeProvenance::as_immutable);

                            let mut ptr = self.mplace_to_ref(&fields_place)?;
                            ptr.layout = self.layout_of(Ty::new_imm_ref(
                                self.tcx.tcx,
                                self.tcx.lifetimes.re_static,
                                fields_layout.ty,
                            ))?;

                            let slice_type = Ty::new_imm_ref(
                                self.tcx.tcx,
                                self.tcx.lifetimes.re_static,
                                Ty::new_slice(self.tcx.tcx, field_type),
                            );
                            let slice_type = self.layout_of(slice_type)?;
                            self.unsize_into(&ptr.into(), slice_type, &mplace.into())?;

                            variant
                        }
                        ty::Uint(_) | ty::Int(_) | ty::Float(_) | ty::Char | ty::Bool => {
                            VariantIdx::from_usize(1)
                        }
                        ty::Adt(_, _)
                        | ty::Foreign(_)
                        | ty::Str
                        | ty::Array(_, _)
                        | ty::Pat(_, _)
                        | ty::Slice(_)
                        | ty::RawPtr(..)
                        | ty::Ref(..)
                        | ty::FnDef(..)
                        | ty::FnPtr(..)
                        | ty::UnsafeBinder(..)
                        | ty::Dynamic(..)
                        | ty::Closure(..)
                        | ty::CoroutineClosure(..)
                        | ty::Coroutine(..)
                        | ty::CoroutineWitness(..)
                        | ty::Never
                        | ty::Alias(..)
                        | ty::Param(_)
                        | ty::Bound(..)
                        | ty::Placeholder(_)
                        | ty::Infer(..)
                        | ty::Error(_) => VariantIdx::from_usize(2),
                    };
                    self.write_discriminant(variant_index, &mplace)?
                }
                sym::size => {
                    let variant_index = if layout.is_sized() {
                        let variant = VariantIdx::from_usize(1);
                        let mplace = self.project_downcast(&mplace, variant)?;
                        let mplace = self.project_field(&mplace, 0)?;
                        self.write_scalar(
                            ScalarInt::try_from_target_usize(layout.size.bytes(), self.tcx.tcx)
                                .unwrap(),
                            &mplace,
                        )?;
                        variant
                    } else {
                        VariantIdx::from_usize(0)
                    };
                    self.write_discriminant(variant_index, &mplace)?;
                }
                other => span_bug!(self.tcx.span, "unknown `Type` field {other}"),
            }
        }

        self.alloc_mark_immutable(mplace.ptr().provenance.unwrap().alloc_id())?;
        interp_ok(mplace)
    }
}
