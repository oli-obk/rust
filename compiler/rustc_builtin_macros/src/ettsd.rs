use rustc_ast::tokenstream::TokenStream;
use rustc_expand::base::{DummyResult, ExpandResult, ExtCtxt, MacroExpanderResult};
use rustc_parse::exp;
use rustc_span::Span;

pub(crate) fn expand<'cx>(
    cx: &'cx mut ExtCtxt<'_>,
    sp: Span,
    tts: TokenStream,
) -> MacroExpanderResult<'cx> {
    let mut parser = cx.new_parser_from_tts(tts);
    let (trait_, args) = match try {
        let trait_ = parser.parse_path(rustc_parse::parser::PathStyle::Mod)?;
        let mut args = vec![];
        parser.expect(exp!(Comma))?;
        loop {
            let arg = parser.parse_path(rustc_parse::parser::PathStyle::Mod)?;
            let res = cx
                .resolver
                .resolve_path(&arg)
                .map_err(|_| cx.dcx().struct_span_err(arg.span, format!("could not resolve")))?;

            args.push((res, arg.span));
            if !parser.eat(exp!(Comma)) {
                break;
            }
        }
        parser.expect(exp!(Eof))?;
        (trait_, args)
    } {
        Ok(val) => val,
        Err(diag) => return ExpandResult::Ready(DummyResult::any(sp, diag.emit_err())),
    };

    let trait_span = trait_.span;
    let Ok(trait_) = cx.resolver.resolve_path(&trait_) else {
        return ExpandResult::Ready(DummyResult::any(
            sp,
            cx.dcx().span_err(trait_.span, "could not resolve"),
        ));
    };

    if cx.resolver.has_impl(trait_, &args, trait_span) {
        ExpandResult::Ready(DummyResult::any(
            sp,
            cx.dcx()
                .struct_span_err(trait_span, "is implemented")
                .with_span_label(args[0].1, "for this type")
                .emit_err(),
        ))
    } else {
        ExpandResult::Ready(DummyResult::any(
            sp,
            cx.dcx()
                .struct_span_err(trait_span, "is not implemented")
                .with_span_label(args[0].1, "for this type")
                .emit_err(),
        ))
    }
}
