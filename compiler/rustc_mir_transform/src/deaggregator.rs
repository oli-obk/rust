use crate::util::expand_aggregate;
use crate::MirPass;
use rustc_middle::mir::*;
use rustc_middle::ty::TyCtxt;

pub struct Deaggregator {
    pub skip_closures: bool,
}

impl<'tcx> MirPass<'tcx> for Deaggregator {
    fn phase_change(&self) -> Option<MirPhase> {
        if self.skip_closures {
            Some(MirPhase::DeaggregatedNonClosures)
        } else {
            Some(MirPhase::Deaggregated)
        }
    }

    fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        let (basic_blocks, local_decls) = body.basic_blocks_and_local_decls_mut();
        let local_decls = &*local_decls;
        for bb in basic_blocks {
            bb.expand_statements(|stmt| {
                // FIXME(eddyb) don't match twice on `stmt.kind` (post-NLL).
                match stmt.kind {
                    // FIXME(#48193) Deaggregate arrays when it's cheaper to do so.
                    StatementKind::Assign(box (
                        _,
                        Rvalue::Aggregate(box AggregateKind::Array(_), _),
                    )) => {
                        return None;
                    }
                    // FIMXE: closures have lots of hacks in borrowck, maybe they will go away with deaggregation?
                    // Either way, skip them for now, as the changes would be big and I want to do them cleanly in
                    // a separate PR.
                    StatementKind::Assign(box (
                        _,
                        Rvalue::Aggregate(box AggregateKind::Closure(..), _),
                    )) if self.skip_closures => return None,
                    StatementKind::Assign(box (_, Rvalue::Aggregate(_, _))) => {}
                    _ => return None,
                }

                let stmt = stmt.replace_nop();
                let source_info = stmt.source_info;
                let StatementKind::Assign(box (lhs, Rvalue::Aggregate(kind, operands))) = stmt.kind else {
                    bug!();
                };

                Some(expand_aggregate(
                    lhs,
                    operands.into_iter().map(|op| {
                        let ty = op.ty(local_decls, tcx);
                        (op, ty)
                    }),
                    *kind,
                    source_info,
                    tcx,
                ))
            });
        }
    }
}
