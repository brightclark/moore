// Copyright (c) 2017 Fabian Schuiki

//! LLHD code generation for VHDL.

use moore_common::score::Result;
use score::*;
use konst::*;
use ty::*;
use num::Signed;
use hir;
use llhd;


/// Generates LLHD code.
pub trait Codegen<I,C> {
	fn codegen(&self, id: I, ctx: &mut C) -> Result<()>;
}


/// This macro implements the `Codegen` trait for a specific combination of
/// identifier and context types.
macro_rules! impl_codegen {
	($slf:tt, $id:ident: $id_ty:ty, $ctx:ident: &mut $ctx_ty:ty => $blk:block) => {
		impl<'sb, 'ast, 'ctx> Codegen<$id_ty, $ctx_ty> for ScoreContext<'sb, 'ast, 'ctx> {
			fn codegen(&$slf, $id: $id_ty, $ctx: &mut $ctx_ty) -> Result<()> $blk
		}
	}
}


impl<'sb, 'ast, 'ctx> ScoreContext<'sb, 'ast, 'ctx> {
	/// Map a VHDL type to the corresponding LLHD type.
	pub fn map_type(&self, ty: &Ty) -> Result<llhd::Type> {
		let ty = self.deref_named_type(ty)?;
		Ok(match *ty {
			Ty::Named(..) => unreachable!(),
			Ty::Null => llhd::void_ty(),
			Ty::Int(ref ty) => {
				let diff = match ty.dir {
					hir::Dir::To => &ty.right_bound - &ty.left_bound,
					hir::Dir::Downto => &ty.left_bound - &ty.right_bound,
				};
				if diff.is_negative() {
					llhd::void_ty()
				} else {
					llhd::int_ty(diff.bits())
				}
			}

			Ty::Enum(ref ty) => {
				let hir = self.hir(ty.decl)?;
				match hir.data {
					Some(hir::TypeData::Enum(_, ref lits)) => llhd::enum_ty(lits.len()),
					_ => unreachable!()
				}
			}

			// Unbounded integers cannot be mapped to LLHD. All cases where
			// such an int can leak through to codegen should actually be caught
			// beforehand in the type check.
			Ty::UnboundedInt => unreachable!(),
		})
	}

	/// Map a constant value to the LLHD counterpart.
	pub fn map_const(&self, konst: &Const) -> Result<llhd::ValueRef> {
		Ok(match *konst {
			// TODO: Map this to llhd::const_void once available.
			Const::Null => llhd::const_int(0, 0.into()),
			Const::Int(ref k) => llhd::const_int(999, k.value.clone()),
			Const::Enum(ref k) => {
				let size = match self.hir(k.decl)?.data {
					Some(hir::TypeData::Enum(_, ref lits)) => lits.len(),
					_ => unreachable!(),
				};
				llhd::const_int(size, k.index.into())
			}
			Const::Float(ref _k) => panic!("cannot map float constant"),
			Const::IntRange(_) | Const::FloatRange(_) => panic!("cannot map range constant"),
		}.into())
	}
}


impl_codegen!(self, id: DeclInBlockRef, ctx: &mut llhd::Entity => {
	match id {
		DeclInBlockRef::Pkg(_id)      => Ok(()),
		DeclInBlockRef::PkgInst(_id)  => Ok(()),
		DeclInBlockRef::Type(_id)     => Ok(()),
		DeclInBlockRef::Subtype(_id)  => Ok(()),
		DeclInBlockRef::Const(id)     => self.codegen(id, ctx),
		DeclInBlockRef::Signal(id)    => self.codegen(id, ctx),
		DeclInBlockRef::SharedVar(id) => self.codegen(id, ctx),
		DeclInBlockRef::File(id)      => self.codegen(id, ctx),
	}
});


impl_codegen!(self, _id: ConstDeclRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});


impl_codegen!(self, id: SignalDeclRef, ctx: &mut llhd::Entity => {
	// Determine the type of the signal.
	let hir = self.existing_hir(id)?;
	let ty = self.ty(id)?;

	// Calculate the initial value for the signal, either from the provided
	// expression or implicitly.
	let init = if let Some(init_id) = hir.init {
		self.const_value(init_id)?
	} else {
		self.default_value_for_type(&ty)?
	};

	println!("signal {:?}, type {:?}, init {:?}", id, ty, init);
	// Create the signal instance.
	let inst = llhd::Inst::new(
		Some(hir.name.value.into()),
		llhd::SignalInst(self.map_type(ty)?, Some(self.map_const(init)?))
	);
	ctx.add_inst(inst, llhd::InstPosition::End);
	Ok(())
});


impl_codegen!(self, _id: SharedVarDeclRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});


impl_codegen!(self, _id: FileDeclRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});

impl_codegen!(self, id: ConcStmtRef, ctx: &mut llhd::Entity => {
	match id {
		ConcStmtRef::Block(id)         => self.codegen(id, ctx),
		ConcStmtRef::Process(id)       => self.codegen(id, ctx),
		ConcStmtRef::ConcProcCall(id)  => self.codegen(id, ctx),
		ConcStmtRef::ConcAssert(id)    => self.codegen(id, ctx),
		ConcStmtRef::ConcSigAssign(id) => self.codegen(id, ctx),
		ConcStmtRef::CompInst(id)      => self.codegen(id, ctx),
		ConcStmtRef::ForGen(id)        => self.codegen(id, ctx),
		ConcStmtRef::IfGen(id)         => self.codegen(id, ctx),
		ConcStmtRef::CaseGen(id)       => self.codegen(id, ctx),
	}
});

impl_codegen!(self, _id: BlockStmtRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});

impl_codegen!(self, id: ProcessStmtRef, ctx: &mut llhd::Entity => {
	let hir = self.hir(id)?;
	let name = match hir.label {
		Some(n) => format!("{}_{}", ctx.name(), n.value),
		None => format!("{}_proc", ctx.name()),
	};
	println!("generating process `{}`", name);
	// TODO: Check which signals are actually read and written.
	let ty = llhd::entity_ty(vec![], vec![]);
	let prok = llhd::Process::new(name, ty.clone());
	// TODO: define the process as a local name
	// TOOD: codegen statements
	// TODO: codegen wait statements implied by sensitivity list
	let prok_ref = self.sb.llmod.borrow_mut().add_process(prok);
	// TODO: wire instantiation with signals in the process' port.
	ctx.add_inst(
		llhd::Inst::new(hir.label.map(|l| l.value.into()), llhd::InstKind::InstanceInst(
			ty, prok_ref.into(), vec![], vec![]
		)),
		llhd::InstPosition::End
	);
	Ok(())
});

impl_codegen!(self, _id: ConcProcCallStmtRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});

impl_codegen!(self, _id: ConcAssertStmtRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});

impl_codegen!(self, _id: ConcSigAssignStmtRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});

impl_codegen!(self, _id: CompInstStmtRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});

impl_codegen!(self, _id: ForGenStmtRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});

impl_codegen!(self, _id: IfGenStmtRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});

impl_codegen!(self, _id: CaseGenStmtRef, _ctx: &mut llhd::Entity => {
	unimplemented!();
});
