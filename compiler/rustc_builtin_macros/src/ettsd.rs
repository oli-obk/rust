use rustc_ast::tokenstream::TokenStream;
use rustc_ast::{self as ast, DUMMY_NODE_ID};
use rustc_expand::base::{
    DummyResult, ExpandResult, ExtCtxt, MacEager, MacroExpanderResult, OptionalInfo,
};
use rustc_parse::exp;
use rustc_span::{Ident, Span, kw, sym};
use smallvec::smallvec;
use thin_vec::thin_vec;

pub(crate) fn expand<'cx>(
    cx: &'cx mut ExtCtxt<'_>,
    span: Span,
    tts: TokenStream,
) -> MacroExpanderResult<'cx> {
    let mut parser = cx.new_parser_from_tts(tts);
    let (name, ty) = match try {
        let name = parser.parse_ident()?;
        parser.expect(exp!(Comma))?;
        let ty = parser.parse_ty()?;
        parser.expect(exp!(Eof))?;
        (name, ty)
    } {
        Ok(val) => val,
        Err(diag) => return ExpandResult::Ready(DummyResult::any(span, diag.emit_err())),
    };

    match cx.resolver.instantiate_optional_info(&ty) {
        Ok(OptionalInfo { needs_drop, is_copy }) => {
            let data_field = Ident::from_str_and_span("data", span);
            let discr_field = Ident::from_str_and_span("discr", span);
            let mut maybe_uninit = cx.std_path(span, &[sym::mem, sym::MaybeUninit]);
            maybe_uninit.segments.last_mut().unwrap().args =
                Some(Box::new(ast::GenericArgs::AngleBracketed(ast::AngleBracketedArgs {
                    span,
                    args: thin_vec![ast::AngleBracketedArg::Arg(ast::GenericArg::Type(ty.clone()))],
                })));
            let mut items = smallvec![];
            // add `repr(C)` attribute
            items.push(cx.item(
                span,
                thin_vec![ast::Attribute {
                    kind: ast::AttrKind::Normal(Box::new(ast::NormalAttr {
                        tokens: None,
                        item: ast::AttrItem {
                            unsafety: ast::Safety::Default,
                            path: ast::Path::from_ident(Ident::new(sym::repr, span)),
                            args: ast::AttrArgs::Delimited(ast::DelimArgs {
                                dspan: ast::tokenstream::DelimSpan::dummy(),
                                delim: ast::token::Delimiter::Parenthesis,
                                tokens: TokenStream::token_alone(
                                    ast::token::TokenKind::Ident(
                                        sym::C,
                                        rustc_ast::token::IdentKind::Normal
                                    ),
                                    span
                                ),
                            }),
                            span
                        }
                    })),
                    id: ast::AttrId::from_u32(0),
                    style: ast::AttrStyle::Outer,
                    span,
                }],
                ast::ItemKind::Struct(
                    name,
                    ast::Generics::default(),
                    ast::VariantData::Struct {
                        fields: thin_vec![
                            ast::FieldDef {
                                attrs: thin_vec![],
                                id: DUMMY_NODE_ID,
                                span,
                                vis: ast::Visibility { kind: ast::VisibilityKind::Inherited, span },
                                extras: None,
                                ident: Some(data_field),
                                ty: cx.ty(span, ast::TyKind::Path(None, maybe_uninit.clone())),
                                is_placeholder: false,
                            },
                            // in C++ this comes after the value
                            ast::FieldDef {
                                attrs: thin_vec![],
                                id: DUMMY_NODE_ID,
                                span,
                                vis: ast::Visibility { kind: ast::VisibilityKind::Inherited, span },
                                extras: None,
                                ident: Some(discr_field),
                                ty: cx.ty(
                                    span,
                                    ast::TyKind::Path(
                                        None,
                                        ast::Path::from_ident(Ident::new(sym::bool, span))
                                    )
                                ),
                                is_placeholder: false,
                            }
                        ],
                        recovered: ast::Recovered::No,
                    },
                ),
            ));
            let self_ty = cx.ty_path(ast::Path::from_ident(name));
            match (needs_drop, is_copy) {
                (true, true) => unreachable!(),
                (true, false) => {
                    items.push(cx.item(
                        span,
                        thin_vec![],
                        ast::ItemKind::Impl(ast::Impl {
                            generics: ast::Generics::default(),
                            constness: ast::Const::No,
                            of_trait: Some(Box::new(ast::TraitImplHeader {
                                defaultness: ast::Defaultness::Implicit,
                                safety: ast::Safety::Default,
                                polarity: ast::ImplPolarity::Positive,
                                trait_ref: ast::TraitRef {
                                    path: cx.std_path(span, &[sym::ops, sym::Drop]),
                                    ref_id: DUMMY_NODE_ID,
                                },
                            })),
                            self_ty,
                            // `fn drop(&mut self) {}`
                            items: thin_vec![Box::new(ast::AssocItem {
                                attrs: thin_vec![],
                                id: DUMMY_NODE_ID,
                                span,
                                vis: ast::Visibility { kind: ast::VisibilityKind::Inherited, span },
                                kind: ast::AssocItemKind::Fn(Box::new(ast::Fn {
                                    defaultness: ast::Defaultness::Implicit,
                                    ident: Ident::new(sym::drop, span),
                                    generics: ast::Generics::default(),
                                    sig: ast::FnSig {
                                        header: ast::FnHeader {
                                            constness: ast::Const::No,
                                            coroutine_marker: None,
                                            safety: ast::Safety::Default,
                                            ext: ast::Extern::None,
                                        },
                                        decl: Box::new(ast::FnDecl {
                                            inputs: thin_vec![ast::Param::from_self(
                                                thin_vec![],
                                                ast::ExplicitSelf {
                                                    node: ast::SelfKind::Region(
                                                        None,
                                                        ast::Mutability::Mut
                                                    ),
                                                    span
                                                },
                                                Ident::new(kw::SelfLower, span)
                                            )],
                                            output: ast::FnRetTy::Default(span)
                                        }),
                                        span
                                    },
                                    contract: None,
                                    define_opaque: None,
                                    body: Some(Box::new(ast::Block {
                                        stmts: thin_vec![ast::Stmt {
                                            id: DUMMY_NODE_ID,
                                            // TODO: optimize with drop_in_place
                                            kind: ast::StmtKind::Semi(cx.expr_method_call(
                                                span,
                                                cx.expr_self(span),
                                                Ident::new(sym::take, span),
                                                thin_vec![]
                                            )),
                                            span
                                        }],
                                        id: DUMMY_NODE_ID,
                                        rules: ast::BlockCheckMode::Default,
                                        span
                                    })),
                                    eii_impl: None
                                })),
                                tokens: None,
                            })],
                        }),
                    ));
                }
                (false, true) => {
                    items.push(cx.item_trait_impl(
                        span,
                        thin_vec![],
                        ast::Generics::default(),
                        ast::Safety::Default,
                        false,
                        ast::TraitRef {
                            path: cx.std_path(span, &[sym::marker, sym::Copy]),
                            ref_id: DUMMY_NODE_ID,
                        },
                        self_ty.clone(),
                        thin_vec![],
                    ));

                    items.push(cx.item_trait_impl(
                        span,
                        thin_vec![],
                        ast::Generics::default(),
                        ast::Safety::Default,
                        false,
                        ast::TraitRef {
                            path: cx.std_path(span, &[sym::clone, sym::Clone]),
                            ref_id: DUMMY_NODE_ID,
                        },
                        self_ty.clone(),
                        // `fn clone(&self) -> Self {}`
                        thin_vec![Box::new(ast::AssocItem {
                            attrs: thin_vec![],
                            id: DUMMY_NODE_ID,
                            span,
                            vis: ast::Visibility { kind: ast::VisibilityKind::Inherited, span },
                            kind: ast::AssocItemKind::Fn(Box::new(ast::Fn {
                                defaultness: ast::Defaultness::Implicit,
                                ident: Ident::new(sym::clone, span),
                                generics: ast::Generics::default(),
                                sig: ast::FnSig {
                                    header: ast::FnHeader {
                                        constness: ast::Const::No,
                                        coroutine_marker: None,
                                        safety: ast::Safety::Default,
                                        ext: ast::Extern::None,
                                    },
                                    decl: Box::new(ast::FnDecl {
                                        inputs: thin_vec![ast::Param::from_self(
                                            thin_vec![],
                                            ast::ExplicitSelf {
                                                node: ast::SelfKind::Region(
                                                    None,
                                                    ast::Mutability::Not
                                                ),
                                                span
                                            },
                                            Ident::new(kw::SelfLower, span)
                                        )],
                                        output: ast::FnRetTy::Ty(self_ty)
                                    }),
                                    span
                                },
                                contract: None,
                                define_opaque: None,
                                body: Some(Box::new(ast::Block {
                                    stmts: thin_vec![ast::Stmt {
                                        id: DUMMY_NODE_ID,
                                        kind: ast::StmtKind::Expr(
                                            cx.expr_deref(span, cx.expr_self(span))
                                        ),
                                        span
                                    }],
                                    id: DUMMY_NODE_ID,
                                    rules: ast::BlockCheckMode::Default,
                                    span
                                })),
                                eii_impl: None
                            })),
                            tokens: None,
                        })],
                    ));
                }
                (false, false) => {}
            }
            ExpandResult::Ready(MacEager::items(items))
        }
        Err(guar) => ExpandResult::Ready(DummyResult::any(span, guar)),
    }
}
