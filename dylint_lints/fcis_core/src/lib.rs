#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::Crate;
use rustc_hir::intravisit::{FnKind, Visitor, walk_poly_trait_ref, walk_ty};
use rustc_hir::{AmbigArg, Body, FnDecl, Item, ItemKind, PolyTraitRef, Ty, TyKind, lang_items};
use rustc_lint::{EarlyContext, EarlyLintPass, LateContext, LateLintPass};
use rustc_span::def_id::LocalDefId;
use rustc_span::{Span, sym};

rustc_session::declare_lint! {
    /// ### What it does
    ///
    /// Requires each crate checked as a functional core to declare `#![no_std]`.
    ///
    /// ### Why is this bad?
    ///
    /// An implicit `std` prelude makes ambient I/O, process, environment, time,
    /// and synchronization APIs available inside the deterministic kernel.
    pub FCIS_CORE_MUST_BE_NO_STD,
    Deny,
    "functional core crates must declare `#![no_std]`"
}

rustc_session::declare_lint! {
    /// ### What it does
    ///
    /// Rejects mutable statics and immutable statics whose types permit
    /// interior mutation in the functional core.
    ///
    /// ### Why is this bad?
    ///
    /// `no_std` code can still observe and modify process-global state through
    /// atomics, cells, locks, and other interior-mutable static values.
    pub FCIS_CORE_MUST_NOT_OWN_AMBIENT_STATE,
    Deny,
    "functional core crates must not own ambient mutable state"
}

rustc_session::declare_lint! {
    /// ### What it does
    ///
    /// Rejects exported function signatures containing function pointers or
    /// `Fn`, `FnMut`, or `FnOnce` bounds.
    ///
    /// ### Why is this bad?
    ///
    /// Callers can use callable inputs to smuggle filesystem, environment,
    /// clock, network, process, or other host-dependent behavior into an API
    /// that is intended to consume complete immutable values.
    pub FCIS_CORE_MUST_NOT_EXPOSE_CALLABLE_INTERFACES,
    Deny,
    "functional core public interfaces must not expose callable behavior"
}

rustc_session::declare_lint_pass!(FcisCoreMustBeNoStd => [FCIS_CORE_MUST_BE_NO_STD]);
rustc_session::declare_lint_pass!(FcisCoreBoundaries => [
    FCIS_CORE_MUST_NOT_OWN_AMBIENT_STATE,
    FCIS_CORE_MUST_NOT_EXPOSE_CALLABLE_INTERFACES,
]);

dylint_linting::dylint_library!();

#[unsafe(no_mangle)]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[
        FCIS_CORE_MUST_BE_NO_STD,
        FCIS_CORE_MUST_NOT_OWN_AMBIENT_STATE,
        FCIS_CORE_MUST_NOT_EXPOSE_CALLABLE_INTERFACES,
    ]);
    lint_store.register_early_pass(|| Box::new(FcisCoreMustBeNoStd));
    lint_store.register_late_pass(|_| Box::new(FcisCoreBoundaries));
}

impl EarlyLintPass for FcisCoreMustBeNoStd {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if krate.attrs.iter().any(|attr| attr.has_name(sym::no_std)) {
            return;
        }

        span_lint_and_help(
            cx,
            FCIS_CORE_MUST_BE_NO_STD,
            krate.spans.inner_span,
            "this functional core crate does not declare `#![no_std]`",
            None,
            "add `#![no_std]` and move filesystem, environment, clock, network, parser, and process access to the imperative shell",
        );
    }
}

impl<'tcx> LateLintPass<'tcx> for FcisCoreBoundaries {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let ItemKind::Static(mutability, ident, _, _) = item.kind else {
            return;
        };

        let ty = cx.tcx.type_of(item.owner_id).instantiate_identity();
        if !mutability.is_mut() && ty.is_freeze(cx.tcx, cx.typing_env()) {
            return;
        }

        span_lint_and_help(
            cx,
            FCIS_CORE_MUST_NOT_OWN_AMBIENT_STATE,
            ident.span,
            "this static gives the functional core ambient mutable state",
            None,
            "pass state into each invocation as an explicit value and return the updated value",
        );
    }

    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        _: &'tcx Body<'tcx>,
        _: Span,
        def_id: LocalDefId,
    ) {
        if matches!(kind, FnKind::Closure) || !cx.effective_visibilities.is_exported(def_id) {
            return;
        }

        let mut visitor = CallableInterfaceVisitor::new(cx);
        visitor.visit_fn_decl(decl);
        let generics = cx
            .tcx
            .hir_get_generics(def_id)
            .expect("exported function items retain HIR generics metadata");
        visitor.visit_generics(generics);

        let Some(span) = visitor.callable_span else {
            return;
        };

        span_lint_and_help(
            cx,
            FCIS_CORE_MUST_NOT_EXPOSE_CALLABLE_INTERFACES,
            span,
            "this exported functional-core interface exposes callable behavior",
            None,
            "accept complete immutable input values instead; keep any deterministic callable helper private and core-owned",
        );
    }
}

struct CallableInterfaceVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    callable_span: Option<Span>,
}

impl<'a, 'tcx> CallableInterfaceVisitor<'a, 'tcx> {
    fn new(cx: &'a LateContext<'tcx>) -> Self {
        Self {
            cx,
            callable_span: None,
        }
    }
}

impl<'tcx> Visitor<'tcx> for CallableInterfaceVisitor<'_, 'tcx> {
    fn visit_ty(&mut self, ty: &'tcx Ty<'_, AmbigArg>) {
        if matches!(ty.kind, TyKind::FnPtr(_)) {
            self.callable_span.get_or_insert(ty.span);
            return;
        }

        walk_ty(self, ty);
    }

    fn visit_poly_trait_ref(&mut self, poly_trait_ref: &'tcx PolyTraitRef<'tcx>) {
        if poly_trait_ref
            .trait_ref
            .trait_def_id()
            .is_some_and(|def_id| {
                lang_items::FN_TRAITS
                    .iter()
                    .any(|&lang_item| self.cx.tcx.lang_items().get(lang_item) == Some(def_id))
            })
        {
            self.callable_span.get_or_insert(poly_trait_ref.span);
            return;
        }

        walk_poly_trait_ref(self, poly_trait_ref);
    }
}
