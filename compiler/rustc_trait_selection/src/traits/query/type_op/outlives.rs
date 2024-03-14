use crate::infer::canonical::{Canonical, CanonicalQueryResponse};
use crate::traits::query::dropck_outlives::{
    compute_dropck_outlives_inner, trivial_dropck_outlives,
};
use crate::traits::ObligationCtxt;
use rustc_hir::def_id::LocalDefId;
use rustc_middle::traits::query::{DropckOutlivesResult, NoSolution};
use rustc_middle::ty::{self, ParamEnvAnd, QueryInput, Ty, TyCtxt};

#[derive(Copy, Clone, Debug, HashStable, TypeFoldable, TypeVisitable)]
pub struct DropckOutlives<'tcx> {
    dropped_ty: Ty<'tcx>,
    defining_opaque_types: &'tcx ty::List<LocalDefId>,
}

impl<'tcx> QueryInput<TyCtxt<'tcx>> for DropckOutlives<'tcx> {
    fn defining_opaque_types(&self) -> &'tcx ty::List<LocalDefId> {
        self.defining_opaque_types
    }
}

impl<'tcx> DropckOutlives<'tcx> {
    pub fn new(dropped_ty: Ty<'tcx>, defining_opaque_types: &'tcx ty::List<LocalDefId>) -> Self {
        DropckOutlives { dropped_ty, defining_opaque_types }
    }
}

impl<'tcx> super::QueryTypeOp<'tcx> for DropckOutlives<'tcx> {
    type QueryResponse = DropckOutlivesResult<'tcx>;

    fn try_fast_path(
        tcx: TyCtxt<'tcx>,
        key: &ParamEnvAnd<'tcx, Self>,
    ) -> Option<Self::QueryResponse> {
        trivial_dropck_outlives(tcx, key.value.dropped_ty).then(DropckOutlivesResult::default)
    }

    fn perform_query(
        tcx: TyCtxt<'tcx>,
        canonicalized: Canonical<'tcx, ParamEnvAnd<'tcx, Self>>,
    ) -> Result<CanonicalQueryResponse<'tcx, Self::QueryResponse>, NoSolution> {
        // FIXME convert to the type expected by the `dropck_outlives`
        // query. This should eventually be fixed by changing the
        // *underlying query*.
        let canonicalized = canonicalized.unchecked_map(|ParamEnvAnd { param_env, value }| {
            let DropckOutlives { dropped_ty, defining_opaque_types } = value;
            param_env.and((defining_opaque_types, dropped_ty))
        });

        tcx.dropck_outlives(canonicalized)
    }

    fn perform_locally_with_next_solver(
        ocx: &ObligationCtxt<'_, 'tcx>,
        key: ParamEnvAnd<'tcx, Self>,
    ) -> Result<Self::QueryResponse, NoSolution> {
        compute_dropck_outlives_inner(ocx, key.param_env.and(key.value.dropped_ty))
    }
}
