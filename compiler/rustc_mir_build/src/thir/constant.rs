use rustc_apfloat::Float;
use rustc_ast as ast;
use rustc_middle::mir::interpret::{LitToConstError, LitToConstInput};
use rustc_middle::ty::{self, ParamEnv, ScalarInt, TyCtxt, ValTree};
use rustc_span::symbol::Symbol;
use rustc_target::abi::Size;

crate fn lit_to_const<'tcx>(
    tcx: TyCtxt<'tcx>,
    lit_input: LitToConstInput<'tcx>,
) -> Result<&'tcx ty::Const<'tcx>, LitToConstError> {
    let LitToConstInput { lit, ty, neg } = lit_input;

    let trunc = |n| {
        let param_ty = ParamEnv::reveal_all().and(ty);
        let width = tcx.layout_of(param_ty).map_err(|_| LitToConstError::Reported)?.size;
        trace!("trunc {} with size {} and shift {}", n, width.bits(), 128 - width.bits());
        let result = width.truncate(n);
        trace!("trunc result: {}", result);
        Ok(ValTree::Leaf(ScalarInt::from_uint(result, width)))
    };

    let byte_array = |bytes: &[u8]| {
        let bytes = bytes
            .iter()
            .copied()
            .map(|b| ValTree::Leaf(ScalarInt::from_uint(b, Size::from_bytes(1))));
        ValTree::Branch(tcx.arena.alloc_from_iter(bytes))
    };

    let val = match (lit, &ty.kind()) {
        (ast::LitKind::Str(s, _), ty::Ref(..)) => ValTree::Str(*s),
        (ast::LitKind::ByteStr(data), ty::Ref(..)) => byte_array(data),
        (ast::LitKind::Byte(n), ty::Uint(ty::UintTy::U8)) => {
            ValTree::Leaf(ScalarInt::from_uint(*n, Size::from_bytes(1)))
        }
        (ast::LitKind::Int(n, _), ty::Uint(_)) | (ast::LitKind::Int(n, _), ty::Int(_)) => {
            trunc(if neg { (*n as i128).overflowing_neg().0 as u128 } else { *n })?
        }
        (ast::LitKind::Float(n, _), ty::Float(fty)) => ValTree::Leaf(
            parse_float(*n, *fty, neg).map_err(|_| LitToConstError::UnparseableFloat)?,
        ),
        (ast::LitKind::Bool(b), ty::Bool) => ValTree::Leaf(ScalarInt::from_bool(*b)),
        (ast::LitKind::Char(c), ty::Char) => ValTree::Leaf(ScalarInt::from_char(*c)),
        (ast::LitKind::Err(_), _) => return Err(LitToConstError::Reported),
        _ => return Err(LitToConstError::TypeError),
    };
    Ok(ty::Const::from_value(tcx, val, ty))
}

fn parse_float(num: Symbol, fty: ty::FloatTy, neg: bool) -> Result<ScalarInt, ()> {
    let num = num.as_str();
    use rustc_apfloat::ieee::{Double, Single};
    match fty {
        ty::FloatTy::F32 => {
            let rust_f = num.parse::<f32>().map_err(|_| ())?;
            let mut f = num.parse::<Single>().unwrap_or_else(|e| {
                panic!("apfloat::ieee::Single failed to parse `{}`: {:?}", num, e)
            });
            assert!(
                u128::from(rust_f.to_bits()) == f.to_bits(),
                "apfloat::ieee::Single gave different result for `{}`: \
                 {}({:#x}) vs Rust's {}({:#x})",
                rust_f,
                f,
                f.to_bits(),
                Single::from_bits(rust_f.to_bits().into()),
                rust_f.to_bits()
            );
            if neg {
                f = -f;
            }
            Ok(ScalarInt::from(f))
        }
        ty::FloatTy::F64 => {
            let rust_f = num.parse::<f64>().map_err(|_| ())?;
            let mut f = num.parse::<Double>().unwrap_or_else(|e| {
                panic!("apfloat::ieee::Double failed to parse `{}`: {:?}", num, e)
            });
            assert!(
                u128::from(rust_f.to_bits()) == f.to_bits(),
                "apfloat::ieee::Double gave different result for `{}`: \
                 {}({:#x}) vs Rust's {}({:#x})",
                rust_f,
                f,
                f.to_bits(),
                Double::from_bits(rust_f.to_bits().into()),
                rust_f.to_bits()
            );
            if neg {
                f = -f;
            }
            Ok(ScalarInt::from(f))
        }
    }
}
