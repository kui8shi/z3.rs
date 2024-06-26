use std::fmt;
use std::rc::Rc;

use z3_sys::*;

use crate::{
    ast::{Ast, Dynamic},
    Context, FuncEntry, FuncInterp,
};

impl FuncInterp {
    pub(crate) unsafe fn wrap(ctx: Rc<Context>, z3_func_interp: Z3_func_interp) -> Self {
        Z3_func_interp_inc_ref(ctx.z3_ctx, z3_func_interp);

        Self {
            ctx,
            z3_func_interp,
        }
    }

    /// Returns the number of arguments in the function interpretation.
    pub fn get_arity(&self) -> usize {
        unsafe { Z3_func_interp_get_arity(self.ctx.z3_ctx, self.z3_func_interp) as usize }
    }

    /// Returns the number of entries in the function interpretation.
    pub fn get_num_entries(&self) -> u32 {
        unsafe { Z3_func_interp_get_num_entries(self.ctx.z3_ctx, self.z3_func_interp) }
    }

    /// Adds an entry to the function interpretation.
    pub fn add_entry(&self, args: &[Dynamic], value: &Dynamic) {
        unsafe {
            let v = Z3_mk_ast_vector(self.ctx.z3_ctx);
            Z3_ast_vector_inc_ref(self.ctx.z3_ctx, v);
            args.iter()
                .for_each(|a| Z3_ast_vector_push(self.ctx.z3_ctx, v, a.z3_ast));

            Z3_func_interp_add_entry(self.ctx.z3_ctx, self.z3_func_interp, v, value.z3_ast);
        }
    }

    /// Returns the entries of the function interpretation.
    pub fn get_entries(&self) -> Vec<FuncEntry> {
        (0..self.get_num_entries())
            .map(|i| unsafe {
                FuncEntry::wrap(
                    self.ctx.clone(),
                    Z3_func_interp_get_entry(self.ctx.z3_ctx, self.z3_func_interp, i),
                )
            })
            .collect()
    }

    /// Returns the else value of the function interpretation.
    /// Returns None if the else value is not set by Z3.
    pub fn get_else(&self) -> Dynamic {
        unsafe {
            Dynamic::wrap(
                self.ctx.clone(),
                Z3_func_interp_get_else(self.ctx.z3_ctx, self.z3_func_interp),
            )
        }
    }

    /// Sets the else value of the function interpretation.
    pub fn set_else(&self, ast: &Dynamic) {
        unsafe { Z3_func_interp_set_else(self.ctx.z3_ctx, self.z3_func_interp, ast.z3_ast) }
    }
}

impl fmt::Display for FuncInterp {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "[")?;
        self.get_entries().into_iter().try_for_each(|e| {
            let n = e.get_num_args();
            if n > 1 {
                write!(f, "[")?;
            };
            write!(
                f,
                "{}",
                e.get_args()
                    .into_iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
            if n > 1 {
                write!(f, "]")?;
            }
            write!(f, " -> {}, ", e.get_value())
        })?;
        write!(f, "else -> {}", self.get_else())?;
        write!(f, "]")
    }
}

impl fmt::Debug for FuncInterp {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl Drop for FuncInterp {
    fn drop(&mut self) {
        unsafe {
            Z3_func_interp_dec_ref(self.ctx.z3_ctx, self.z3_func_interp);
        }
    }
}
