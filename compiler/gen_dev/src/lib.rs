#![warn(clippy::all, clippy::dbg_macro)]
// I'm skeptical that clippy:large_enum_variant is a good lint to have globally enabled.
//
// It warns about a performance problem where the only quick remediation is
// to allocate more on the heap, which has lots of tradeoffs - including making it
// long-term unclear which allocations *need* to happen for compilation's sake
// (e.g. recursive structures) versus those which were only added to appease clippy.
//
// Effectively optimizing data struture memory layout isn't a quick fix,
// and encouraging shortcuts here creates bad incentives. I would rather temporarily
// re-enable this when working on performance optimizations than have it block PRs.
#![allow(clippy::large_enum_variant)]

use bumpalo::{collections::Vec, Bump};
use object::write::Object;
use roc_collections::all::{MutMap, MutSet};
use roc_module::symbol::{Interns, Symbol};
use roc_mono::ir::{Expr, Literal, Proc, Stmt};
use roc_mono::layout::Layout;
use target_lexicon::{BinaryFormat, Triple};

pub mod elf;
pub mod run_roc;
pub mod x86_64;

pub struct Env<'a> {
    pub arena: &'a Bump,
    pub interns: Interns,
    pub exposed_to_host: MutSet<Symbol>,
}

/// build_module is the high level builder/delegator.
/// It takes the request to build a module and output the object file for the module.
pub fn build_module<'a>(
    env: &'a Env,
    target: &Triple,
    procedures: MutMap<(Symbol, Layout<'a>), Proc<'a>>,
) -> Result<Object, String> {
    match target.binary_format {
        BinaryFormat::Elf => elf::build_module(env, target, procedures),
        x => Err(format! {
        "the binary format, {:?}, is not yet implemented",
        x}),
    }
}

trait Backend<'a> {
    /// new creates a new backend that will output to the specific Object.
    fn new(env: &'a Env) -> Self;

    fn env(&self) -> &'a Env<'a>;

    /// reset resets any registers or other values that may be occupied at the end of a procedure.
    fn reset(&mut self);

    /// build_proc creates a procedure and outputs it to the wrapped object writer.
    /// This will need to return the list of relocations because they have to be added differently based on file format.
    /// Also, assembly will of course be generated by individual calls on backend like may setup_stack.
    fn build_proc(&mut self, proc: Proc<'a>) -> &'a [u8] {
        self.reset();
        // TODO: let the backend know of all the arguments.
        let mut buf = bumpalo::vec!(in self.env().arena);
        self.build_stmt(&mut buf, &proc.body);
        self.wrap_proc(buf).into_bump_slice()
    }

    /// wrap_proc does any setup and cleanup that should happen around the procedure.
    /// For example, this can store the frame pionter and setup stack space.
    /// wrap_proc is run at the end of build_proc when all internal code is finalized.
    /// wrap_proc returns a Vec because it is expected to prepend data.
    fn wrap_proc(&mut self, buf: Vec<'a, u8>) -> Vec<'a, u8>;

    /// build_stmt builds a statement and outputs at the end of the buffer.
    fn build_stmt(&mut self, buf: &mut Vec<'a, u8>, stmt: &Stmt<'a>) {
        match stmt {
            Stmt::Let(sym, expr, layout, following) => {
                self.build_expr(buf, sym, expr, layout);
                self.build_stmt(buf, following);
            }
            Stmt::Ret(sym) => {
                self.return_symbol(buf, sym);
            }
            x => unimplemented!("the statement, {:?}, is not yet implemented", x),
        }
    }

    /// build_expr builds the expressions for the specified symbol.
    /// The builder must keep track of the symbol because it may be refered to later.
    /// In many cases values can be lazy loaded, like literals.
    fn build_expr(
        &mut self,
        _buf: &mut Vec<'a, u8>,
        sym: &Symbol,
        expr: &Expr<'a>,
        layout: &Layout<'a>,
    ) {
        match expr {
            Expr::Literal(lit) => {
                self.set_symbol_to_lit(sym, lit, layout);
            }
            x => unimplemented!("the expression, {:?}, is not yet implemented", x),
        }
    }

    /// set_symbol_to_lit sets a symbol to be equal to a literal.
    /// When the symbol is used, the literal should be loaded.
    fn set_symbol_to_lit(&mut self, sym: &Symbol, lit: &Literal<'a>, layout: &Layout<'a>);

    /// return_symbol moves a symbol to the correct return location for the backend.
    fn return_symbol(&mut self, buf: &mut Vec<'a, u8>, sym: &Symbol);
}
