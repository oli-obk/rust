use rustc_ast::tokenstream::TokenStream;
use rustc_expand::base::{DummyResult, ExpandResult, ExtCtxt, MacroExpanderResult};
use rustc_span::Span;

pub(crate) fn expand<'cx>(
    cx: &'cx mut ExtCtxt<'_>,
    sp: Span,
    tts: TokenStream,
) -> MacroExpanderResult<'cx> {
    let mut parser = cx.new_parser_from_tts(tts);
    let path = match parser.parse_path(rustc_parse::parser::PathStyle::Mod) {
        Ok(parsed) => parsed,
        Err(err) => {
            return ExpandResult::Ready(DummyResult::any(sp, err.emit_err()));
        }
    };
    let Ok(id) = cx.resolver.resolve_path(&path) else {
        return ExpandResult::Ready(DummyResult::any(
            sp,
            cx.dcx().span_err(path.span, "could not resolve"),
        ));
    };

    if cx.resolver.has_eq_impl(id, sp) {
        ExpandResult::Ready(DummyResult::any(sp, cx.dcx().span_err(path.span, "has eq impl")))
    } else {
        ExpandResult::Ready(DummyResult::any(
            sp,
            cx.dcx().span_err(path.span, "does not have eq impl"),
        ))
    }
}
