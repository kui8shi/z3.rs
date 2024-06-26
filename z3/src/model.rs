use std::ffi::CStr;
use std::fmt;
use std::rc::Rc;

use z3_sys::*;

use crate::{ast::Ast, Context, FuncDecl, FuncInterp, Model, Optimize, Solver};

impl Model {
    unsafe fn wrap(ctx: Rc<Context>, z3_mdl: Z3_model) -> Model {
        Z3_model_inc_ref(ctx.z3_ctx, z3_mdl);
        Model { ctx, z3_mdl }
    }

    pub fn of_solver(slv: &Solver) -> Option<Model> {
        unsafe {
            let m = Z3_solver_get_model(slv.ctx.z3_ctx, slv.z3_slv);
            if m.is_null() {
                None
            } else {
                Some(Self::wrap(slv.ctx.clone(), m))
            }
        }
    }

    pub fn of_optimize(opt: &Optimize) -> Option<Model> {
        unsafe {
            let m = Z3_optimize_get_model(opt.ctx.z3_ctx, opt.z3_opt);
            if m.is_null() {
                None
            } else {
                Some(Self::wrap(opt.ctx.clone(), m))
            }
        }
    }

    /// Translate model to context `dest`
    pub fn translate(&self, dest: Rc<Context>) -> Model {
        unsafe {
            let model = Z3_model_translate(self.ctx.z3_ctx, self.z3_mdl, dest.z3_ctx);
            Model::wrap(dest, model)
        }
    }

    /// Returns the interpretation of the given `ast` in the `Model`
    /// Returns `None` if there is no interpretation in the `Model`
    pub fn get_const_interp<T: Ast>(&self, ast: &T) -> Option<T> {
        let func = ast.safe_decl().ok()?;

        let ret =
            unsafe { Z3_model_get_const_interp(self.ctx.z3_ctx, self.z3_mdl, func.z3_func_decl) };
        if ret.is_null() {
            None
        } else {
            Some(unsafe { T::wrap(self.ctx.clone(), ret) })
        }
    }

    /// Returns the interpretation of the given `f` in the `Model`
    /// Returns `None` if arity > 0, or there is no interpretation in the `Model`
    pub fn get_func_interp_as_const<T: Ast>(&self, f: &FuncDecl) -> Option<T> {
        if f.arity() == 0 {
            let ret =
                unsafe { Z3_model_get_const_interp(self.ctx.z3_ctx, self.z3_mdl, f.z3_func_decl) };
            if ret.is_null() {
                None
            } else {
                Some(unsafe { T::wrap(self.ctx.clone(), ret) })
            }
        } else {
            None
        }
    }

    /// Returns the interpretation of the given `f` in the `Model`
    /// Returns `None` if there is no interpretation in the `Model`
    pub fn get_func_interp(&self, f: &FuncDecl) -> Option<FuncInterp> {
        if f.arity() == 0 {
            let ret =
                unsafe { Z3_model_get_const_interp(self.ctx.z3_ctx, self.z3_mdl, f.z3_func_decl) };
            if ret.is_null() {
                None
            } else {
                let sort_kind = unsafe {
                    Z3_get_sort_kind(
                        self.ctx.z3_ctx,
                        Z3_get_range(self.ctx.z3_ctx, f.z3_func_decl),
                    )
                };
                match sort_kind {
                    SortKind::Array => {
                        if unsafe { Z3_is_as_array(self.ctx.z3_ctx, ret) } {
                            let fd = unsafe {
                                FuncDecl::wrap(
                                    self.ctx.clone(),
                                    Z3_get_as_array_func_decl(self.ctx.z3_ctx, ret),
                                )
                            };
                            self.get_func_interp(&fd)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
        } else {
            let ret =
                unsafe { Z3_model_get_func_interp(self.ctx.z3_ctx, self.z3_mdl, f.z3_func_decl) };
            if ret.is_null() {
                None
            } else {
                Some(unsafe { FuncInterp::wrap(self.ctx.clone(), ret) })
            }
        }
    }

    pub fn eval<T>(&self, ast: &T, model_completion: bool) -> Option<T>
    where
        T: Ast,
    {
        let mut tmp: Z3_ast = ast.get_z3_ast();
        let res = {
            unsafe {
                Z3_model_eval(
                    self.ctx.z3_ctx,
                    self.z3_mdl,
                    ast.get_z3_ast(),
                    model_completion,
                    &mut tmp,
                )
            }
        };
        if res {
            Some(unsafe { T::wrap(self.ctx.clone(), tmp) })
        } else {
            None
        }
    }

    fn len(&self) -> u32 {
        unsafe {
            Z3_model_get_num_consts(self.ctx.z3_ctx, self.z3_mdl)
                + Z3_model_get_num_funcs(self.ctx.z3_ctx, self.z3_mdl)
        }
    }

    pub fn iter(&self) -> ModelIter {
        self.into_iter()
    }
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let p = unsafe { Z3_model_to_string(self.ctx.z3_ctx, self.z3_mdl) };
        if p.is_null() {
            return Result::Err(fmt::Error);
        }
        match unsafe { CStr::from_ptr(p) }.to_str() {
            Ok(s) => write!(f, "{s}"),
            Err(_) => Result::Err(fmt::Error),
        }
    }
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe { Z3_model_dec_ref(self.ctx.z3_ctx, self.z3_mdl) };
    }
}

#[derive(Debug)]
/// <https://z3prover.github.io/api/html/classz3py_1_1_model_ref.html#a7890b7c9bc70cf2a26a343c22d2c8367>
pub struct ModelIter<'ctx> {
    model: &'ctx Model,
    idx: u32,
    len: u32,
}

impl<'ctx> IntoIterator for &'ctx Model {
    type Item = FuncDecl;
    type IntoIter = ModelIter<'ctx>;

    fn into_iter(self) -> Self::IntoIter {
        ModelIter {
            model: self,
            idx: 0,
            len: self.len(),
        }
    }
}

impl<'ctx> Iterator for ModelIter<'ctx> {
    type Item = FuncDecl;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            None
        } else {
            let num_consts =
                unsafe { Z3_model_get_num_consts(self.model.ctx.z3_ctx, self.model.z3_mdl) };
            if self.idx < num_consts {
                let const_decl = unsafe {
                    Z3_model_get_const_decl(self.model.ctx.z3_ctx, self.model.z3_mdl, self.idx)
                };
                self.idx += 1;
                Some(unsafe { FuncDecl::wrap(self.model.ctx.clone(), const_decl) })
            } else {
                let func_decl = unsafe {
                    Z3_model_get_func_decl(
                        self.model.ctx.z3_ctx,
                        self.model.z3_mdl,
                        self.idx - num_consts,
                    )
                };
                self.idx += 1;
                Some(unsafe { FuncDecl::wrap(self.model.ctx.clone(), func_decl) })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.len - self.idx) as usize;
        (len, Some(len))
    }
}

#[test]
fn test_unsat() {
    use crate::{ast, Config, SatResult};
    let cfg = Config::new();
    let ctx = Rc::new(Context::new(&cfg));
    let solver = Solver::new(ctx.clone());
    solver.assert(&ast::Bool::from_bool(ctx.clone(), false));
    assert_eq!(solver.check(), SatResult::Unsat);
    assert!(solver.get_model().is_none());
}
