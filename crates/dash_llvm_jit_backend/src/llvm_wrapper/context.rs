use std::ffi::CStr;
use std::ffi::CString;
use std::fmt::format;

use dash_typed_cfg::passes::type_infer::Type;
use llvm_sys::core::LLVMAppendBasicBlockInContext;
use llvm_sys::core::LLVMConstInt;
use llvm_sys::core::LLVMConstReal;
use llvm_sys::core::LLVMContextCreate;
use llvm_sys::core::LLVMCreateBuilderInContext;
use llvm_sys::core::LLVMDoubleTypeInContext;
use llvm_sys::core::LLVMFunctionType;
use llvm_sys::core::LLVMInt1TypeInContext;
use llvm_sys::core::LLVMInt32TypeInContext;
use llvm_sys::core::LLVMInt64TypeInContext;
use llvm_sys::core::LLVMInt8TypeInContext;
use llvm_sys::core::LLVMModuleCreateWithNameInContext;
use llvm_sys::core::LLVMPointerType;
use llvm_sys::core::LLVMStructTypeInContext;
use llvm_sys::core::LLVMVoidTypeInContext;
use llvm_sys::prelude::LLVMContextRef;

use super::module::Module;
use super::raw;
use super::BasicBlock;
use super::Builder;
use super::Function;
use super::Ty;
use super::Value;

pub struct Context {
    module_count: usize,
    cx: LLVMContextRef,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            module_count: 0,
            cx: unsafe { LLVMContextCreate() },
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            module_count: 0,
            cx: unsafe { LLVMContextCreate() },
        }
    }

    pub fn create_module_with_name(&mut self, name: &CStr) -> Module {
        self.module_count += 1;
        Module(unsafe { LLVMModuleCreateWithNameInContext(name.as_ptr(), self.cx) })
    }

    pub fn create_module(&mut self) -> Module {
        self.create_module_with_name(CStr::from_bytes_with_nul(b"anon\0").unwrap())
    }

    pub fn i1_ty(&self) -> Ty {
        Ty(unsafe { LLVMInt1TypeInContext(self.cx) })
    }

    pub fn i8_ty(&self) -> Ty {
        Ty(unsafe { LLVMInt8TypeInContext(self.cx) })
    }

    pub fn i32_ty(&self) -> Ty {
        Ty(unsafe { LLVMInt32TypeInContext(self.cx) })
    }

    pub fn i64_ty(&self) -> Ty {
        Ty(unsafe { LLVMInt64TypeInContext(self.cx) })
    }

    pub fn pointer_ty(&self, inner: &Ty) -> Ty {
        Ty(unsafe { LLVMPointerType(inner.0, 0) })
    }

    pub fn void_ty(&self) -> Ty {
        Ty(unsafe { LLVMVoidTypeInContext(self.cx) })
    }

    pub fn function_ty(&self, ret: &Ty, params: &mut [Ty]) -> Ty {
        Ty(unsafe {
            LLVMFunctionType(
                ret.0,
                Ty::slice_of_tys_as_raw(params).as_mut_ptr(),
                params.len().try_into().unwrap(),
                0,
            )
        })
    }

    pub fn const_i1(&self, val: bool) -> Value {
        Value(unsafe { LLVMConstInt(self.i1_ty().0, val as u64, 0) })
    }

    pub fn const_i32(&self, val: i32) -> Value {
        Value(unsafe { LLVMConstInt(self.i32_ty().0, val as u64, 0) })
    }

    pub fn const_i64(&self, val: i64) -> Value {
        Value(unsafe { LLVMConstInt(self.i64_ty().0, val as u64, 0) })
    }

    pub fn const_f64(&self, val: f64) -> Value {
        Value(unsafe { LLVMConstReal(self.f64_ty().0, val) })
    }

    pub fn f64_ty(&self) -> Ty {
        Ty(unsafe { LLVMDoubleTypeInContext(self.cx) })
    }

    pub fn mir_ty_to_llvm_ty(&self, mir: &Type) -> Ty {
        match mir {
            Type::Boolean => self.i1_ty(),
            Type::F64 => self.f64_ty(),
            Type::I64 => self.i64_ty(),
        }
    }

    pub fn struct_ty_unpacked(&self, tys: &mut [Ty]) -> Ty {
        let tys = Ty::slice_of_tys_as_raw(tys);
        Ty(unsafe { LLVMStructTypeInContext(self.cx, tys.as_mut_ptr(), tys.len().try_into().unwrap(), 0) })
    }

    pub fn create_builder(&self) -> Builder {
        Builder(unsafe { LLVMCreateBuilderInContext(self.cx) })
    }

    pub fn append_basic_block(&self, function: &Function, name: &CStr) -> BasicBlock {
        BasicBlock(unsafe { LLVMAppendBasicBlockInContext(self.cx, function.as_ptr(), name.as_ptr()) })
    }
}
