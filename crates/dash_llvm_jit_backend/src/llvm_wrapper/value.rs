use llvm_sys::core::LLVMAddIncoming;
use llvm_sys::core::LLVMGetTypeKind;
use llvm_sys::core::LLVMTypeOf;
use llvm_sys::prelude::LLVMTypeRef;
use llvm_sys::prelude::LLVMValueRef;
use llvm_sys::LLVMTypeKind;

use crate::util::transmute_slice_mut;

use super::BasicBlock;
use super::Ty;

#[derive(Clone)]
pub struct Value(pub(super) LLVMValueRef);

impl Value {
    pub fn slice_of_values_as_raw(slice: &mut [Value]) -> &mut [LLVMValueRef] {
        unsafe { transmute_slice_mut(slice) }
    }

    pub fn ty(&self) -> Ty {
        Ty(unsafe { LLVMTypeOf(self.0) })
    }

    pub fn ty_kind(&self) -> LLVMTypeKind {
        self.ty().kind()
    }
}

pub struct Phi(pub(super) Value);

impl Phi {
    pub fn add_incoming(&self, value: &Value, block: &BasicBlock) {
        let mut values = [value.0];
        let mut blocks = [block.0];
        unsafe { LLVMAddIncoming(self.0 .0, values.as_mut_ptr(), blocks.as_mut_ptr(), 1) };
    }

    pub fn as_value(&self) -> &Value {
        &self.0
    }
}
