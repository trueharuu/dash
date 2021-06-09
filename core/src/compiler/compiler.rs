// This file is cursed. You've been warned
use crate::{
    compiler::agent::ImportResult,
    parser::{
        expr::{
            ArrayLiteral, AssignmentExpr, BinaryExpr, ConditionalExpr, Expr, FunctionCall,
            GroupingExpr, LiteralExpr, ObjectLiteral, Postfix, PropertyAccessExpr, Seq, UnaryExpr,
        },
        statement::{
            BlockStatement, ExportKind, ForLoop, FunctionDeclaration, IfStatement, ImportKind,
            ReturnStatement, SpecifierKind, Statement, TryCatch, VariableDeclaration,
            VariableDeclarationKind, WhileLoop,
        },
        token::TokenType,
    },
    util::MaybeOwned,
    visitor::Visitor,
    vm::{
        instruction::{Constant, Instruction, Opcode},
        stack::{IteratorOrder, Stack},
        value::{
            function::{Constructor, FunctionType, Module, UserFunction},
            Value, ValueKind,
        },
    },
};
use std::{borrow::Cow, convert::TryFrom, ptr::NonNull};

use super::{
    agent::Agent,
    scope::{Local, ScopeGuard},
    upvalue::Upvalue,
};

pub type Ast<'a> = Vec<Statement<'a>>;

#[derive(Debug)]
pub struct Compiler<'a, A> {
    ast: Option<Ast<'a>>,
    top: Option<NonNull<Compiler<'a, A>>>,
    upvalues: Stack<Upvalue, 1024>,
    scope: ScopeGuard<Local<'a>, 1024>,
    agent: Option<MaybeOwned<A>>,
    is_module: bool,
}

pub struct CompileResult {
    pub instructions: Vec<Instruction>,
    pub upvalues: Stack<Upvalue, 1024>,
}

impl<'a, A: Agent> Compiler<'a, A> {
    pub fn new(ast: Ast<'a>, agent: Option<MaybeOwned<A>>, is_module: bool) -> Self {
        let scope = ScopeGuard::new();
        Self {
            ast: Some(ast),
            top: None,
            upvalues: Stack::new(),
            agent,
            scope,
            is_module,
        }
    }

    pub fn with_scopeguard<'b>(
        ast: Ast<'a>,
        scope: ScopeGuard<Local<'a>, 1024>,
        agent: Option<MaybeOwned<A>>,
        caller: Option<NonNull<Compiler<'a, A>>>,
        is_module: bool,
    ) -> Self {
        Self {
            ast: Some(ast),
            upvalues: Stack::new(),
            top: caller,
            agent,
            scope,
            is_module,
        }
    }

    pub unsafe fn caller(&self) -> Option<&Compiler<'a, A>> {
        self.top.as_ref().map(|t| t.as_ref())
    }

    pub unsafe fn caller_mut(&mut self) -> Option<&mut Compiler<'a, A>> {
        self.top.as_mut().map(|t| t.as_mut())
    }

    pub unsafe fn find_upvalue(&mut self, name: &'a [u8]) -> Option<usize> {
        let top = self.caller_mut()?;

        if let Some(idx) = top.scope.find_variable(name) {
            return Some(self.add_upvalue(Upvalue::new(true, idx)));
        }

        if let Some(idx) = top.find_upvalue(name) {
            return Some(self.add_upvalue(Upvalue::new(false, idx)));
        }

        None
    }

    pub fn add_upvalue(&mut self, value: Upvalue) -> usize {
        if let Some((idx, _)) = self.upvalues.find(|&x| x == value) {
            return idx;
        }

        self.upvalues.push(value);
        self.upvalues.get_stack_pointer() - 1
    }

    pub fn compile(self) -> Result<Vec<Instruction>, CompileError<'a>> {
        let is_top = self.top.is_none();
        let is_module = self.is_module;
        let mut instructions = self.compile_frame()?.instructions;

        if is_top {
            if let Some(last) = instructions.last() {
                if matches!(last, Instruction::Op(Opcode::Pop)) {
                    instructions.pop();
                }
            }

            if is_module {
                instructions.push(Instruction::Op(Opcode::ReturnModule));
            } else {
                instructions.push(Instruction::Op(Opcode::Return));
            }
        }

        Ok(instructions)
    }

    fn compile_frame(mut self) -> Result<CompileResult, CompileError<'a>> {
        let mut instructions = Vec::new();

        let statements = self.ast.take().unwrap();

        for statement in statements {
            for instruction in self.accept(&statement)? {
                instructions.push(instruction);
            }
        }

        Ok(CompileResult {
            instructions,
            upvalues: self.upvalues,
        })
    }

    fn compile_variable_declaration(
        &mut self,
        var: &VariableDeclaration<'a>,
        value: Vec<Instruction>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let has_value = !value.is_empty() || var.value.is_some();

        let mut instructions = value;

        let global = self.scope.is_global();

        let (op_with_value, op_no_value) = if global {
            (Opcode::SetGlobal, Opcode::SetGlobalNoValue)
        } else {
            (Opcode::SetLocal, Opcode::SetLocalNoValue)
        };

        if !global {
            let stack_idx = self
                .scope
                .push_local(Local::new(var.name, self.scope.depth));
            instructions.push(Instruction::Op(Opcode::Constant));
            instructions.push(Instruction::Operand(Constant::Index(stack_idx)));
        } else {
            instructions.push(Instruction::Op(Opcode::Constant));
            instructions.push(Instruction::Operand(Constant::Identifier(
                std::str::from_utf8(var.name).unwrap().to_owned(),
            )));
        }

        if has_value {
            instructions.push(Instruction::Op(op_with_value));
        } else {
            instructions.push(Instruction::Op(op_no_value));
        }

        Ok(instructions)
    }
}

#[derive(Debug)]
pub enum CompileError<'a> {
    ModuleNotFound(&'a [u8]),
    NotImplemented(&'static str),
    ImportDisabled,
    NativeImportFailed,
}

impl<'a> CompileError<'a> {
    pub fn to_string(&self) -> Cow<str> {
        match self {
            Self::ModuleNotFound(mo) => Cow::Owned(format!(
                "Failed to resolve module specifier {}",
                std::str::from_utf8(mo).unwrap()
            )),
            Self::NotImplemented(cause) => Cow::Borrowed(cause),
            Self::ImportDisabled => Cow::Borrowed("Imports are disabled for this context"),
            Self::NativeImportFailed => Cow::Borrowed("Native import failed"),
        }
    }
}

impl<'a, A: Agent> Visitor<'a, Result<Vec<Instruction>, CompileError<'a>>> for Compiler<'a, A> {
    fn visit_literal_expression(
        &mut self,
        e: &LiteralExpr<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = Vec::with_capacity(3);
        instructions.push(Instruction::Op(Opcode::Constant));
        let value = match e {
            LiteralExpr::Identifier(ident) => {
                Constant::Identifier(std::str::from_utf8(ident).unwrap().to_owned())
            }
            other => Constant::JsValue(other.to_value()),
        };

        if let LiteralExpr::Identifier(ident) = e {
            match *ident {
                b"this" => {
                    instructions[0] = Instruction::Op(Opcode::GetThis);
                    return Ok(instructions);
                }
                b"super" => {
                    instructions[0] = Instruction::Op(Opcode::GetSuper);
                    return Ok(instructions);
                }
                b"globalThis" => {
                    instructions[0] = Instruction::Op(Opcode::GetGlobalThis);
                    return Ok(instructions);
                }
                _ => {}
            };

            if !self.scope.is_global() {
                let stack_idx = self.scope.find_variable(ident);

                if let Some(stack_idx) = stack_idx {
                    instructions.push(Instruction::Operand(Constant::Index(stack_idx)));
                    instructions.push(Instruction::Op(Opcode::GetLocal));
                    return Ok(instructions);
                }
            }

            if let Some(idx) = unsafe { self.find_upvalue(ident) } {
                instructions.push(Instruction::Operand(Constant::Index(idx)));
                instructions.push(Instruction::Op(Opcode::GetUpvalue));
                return Ok(instructions);
            }

            instructions.push(Instruction::Operand(value));
            instructions.push(Instruction::Op(Opcode::GetGlobal));
        } else {
            instructions.push(Instruction::Operand(value));
        }

        Ok(instructions)
    }

    fn visit_binary_expression(
        &mut self,
        e: &BinaryExpr<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&e.left)?;

        // Will stay -1 if it's not &&, || or ??
        let mut jmp_idx: isize = -1;

        match e.operator {
            TokenType::LogicalAnd | TokenType::LogicalOr | TokenType::NullishCoalescing => {
                let ty = e.operator;

                instructions.push(Instruction::Op(Opcode::Constant));
                jmp_idx = isize::try_from(instructions.len()).unwrap();
                instructions.push(Instruction::Op(Opcode::Nop));

                match ty {
                    TokenType::LogicalAnd => {
                        instructions.push(Instruction::Op(Opcode::ShortJmpIfFalse))
                    }
                    TokenType::LogicalOr => {
                        instructions.push(Instruction::Op(Opcode::ShortJmpIfTrue))
                    }
                    TokenType::NullishCoalescing => {
                        instructions.push(Instruction::Op(Opcode::ShortJmpIfNullish))
                    }
                    _ => {}
                };

                instructions.push(Instruction::Op(Opcode::Pop));
            }
            _ => {}
        };

        let right = self.accept_expr(&e.right)?;
        instructions.extend(right);

        if jmp_idx > -1 {
            let jmp_idx = jmp_idx as usize;

            let instruction_count = instructions.len() - jmp_idx - 2;
            instructions[jmp_idx] = Instruction::Operand(Constant::Index(instruction_count));
        } else {
            instructions.push(Instruction::Op(e.operator.into()));
        }

        Ok(instructions)
    }

    fn visit_while_loop(
        &mut self,
        l: &WhileLoop<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&l.condition)?;

        instructions.push(Instruction::Op(Opcode::Constant));
        let jmp_idx = instructions.len();
        instructions.push(Instruction::Op(Opcode::Nop));

        instructions.push(Instruction::Op(Opcode::ShortJmpIfFalse));
        instructions.push(Instruction::Op(Opcode::Pop));

        // Compile body
        instructions.extend(self.accept(&l.body)?);

        let instruction_count_ = instructions.len() - jmp_idx + 1;
        let instruction_count = Instruction::Operand(Constant::Index(instruction_count_));
        instructions[jmp_idx] = instruction_count;

        // Emit backjump to evaluate condition
        instructions.push(Instruction::Op(Opcode::Constant));
        let backjmp_count = instruction_count_ + jmp_idx + 2;
        instructions.push(Instruction::Operand(Constant::Index(backjmp_count)));
        instructions.push(Instruction::Op(Opcode::BackJmp));
        instructions.push(Instruction::Op(Opcode::Pop));

        Ok(instructions)
    }

    fn visit_grouping_expression(
        &mut self,
        e: &GroupingExpr<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        self.accept_expr(&e.0)
    }

    fn visit_unary_expression(
        &mut self,
        e: &UnaryExpr<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&e.expr)?;

        match e.operator {
            TokenType::Plus => instructions.push(Instruction::Op(Opcode::Positive)),
            TokenType::Minus => instructions.push(Instruction::Op(Opcode::Negate)),
            TokenType::Typeof => instructions.push(Instruction::Op(Opcode::Typeof)),
            TokenType::LogicalNot => instructions.push(Instruction::Op(Opcode::LogicalNot)),
            TokenType::Void => instructions.push(Instruction::Op(Opcode::Void)),
            _ => todo!(),
        }

        Ok(instructions)
    }

    fn visit_variable_declaration(
        &mut self,
        v: &VariableDeclaration<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let value = if let Some(value) = &v.value {
            self.accept_expr(value)?
        } else {
            Vec::new()
        };

        self.compile_variable_declaration(v, value)
    }

    fn visit_if_statement(
        &mut self,
        i: &IfStatement<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&i.condition)?;

        instructions.push(Instruction::Op(Opcode::Constant));
        let jmp_idx = instructions.len();
        instructions.push(Instruction::Op(Opcode::Nop));
        instructions.push(Instruction::Op(Opcode::ShortJmpIfFalse));
        instructions.push(Instruction::Op(Opcode::Pop));

        let then_instructions = self.accept(&i.then)?;
        instructions[jmp_idx] = Instruction::Operand(Constant::Index(then_instructions.len() + 1));

        instructions.extend(then_instructions);

        let mut jumps: Vec<(usize, usize, usize)> = Vec::new();

        // For simplicitly, we desugar the last `else` to another `else if` branch
        // with `true` as condition
        if let Some(then) = &i.el {
            let mut branches = i.branches.borrow_mut();
            branches.push(IfStatement::new(
                Expr::bool_literal(true),
                *then.clone(),
                Vec::new(),
                None,
            ));
        }

        for branch in i.branches.borrow().iter() {
            let old_count = instructions.len();

            let mut branch_instructions = self.accept_expr(&branch.condition)?;

            branch_instructions.push(Instruction::Op(Opcode::Constant));
            let condition_out_jmp_offset = branch_instructions.len();
            branch_instructions.push(Instruction::Op(Opcode::Nop));
            branch_instructions.push(Instruction::Op(Opcode::ShortJmpIfFalse));
            branch_instructions.push(Instruction::Op(Opcode::Pop));

            branch_instructions.extend(self.accept(&branch.then)?);

            branch_instructions.push(Instruction::Op(Opcode::Constant));
            let final_out_jmp_offset = branch_instructions.len();
            branch_instructions.push(Instruction::Op(Opcode::Nop));
            branch_instructions.push(Instruction::Op(Opcode::ShortJmpIfFalse));
            branch_instructions.push(Instruction::Op(Opcode::Pop));

            instructions.extend(branch_instructions);

            jumps.push((old_count, condition_out_jmp_offset, final_out_jmp_offset));
        }

        let instruction_count = instructions.len();

        for idx in 0..jumps.len() {
            let current = jumps[idx];

            instructions[current.0 + current.1] =
                Instruction::Operand(Constant::Index(current.2 - current.1));

            instructions[current.0 + current.2] = Instruction::Operand(Constant::Index(
                instruction_count - (current.0 + current.2) - 3,
            ));
        }

        Ok(instructions)
    }

    fn visit_block_statement(
        &mut self,
        b: &BlockStatement<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        self.scope.enter_scope();
        let mut instructions = Vec::new();

        for stmt in &b.0 {
            instructions.extend(self.accept(stmt)?);
        }

        self.scope.leave_scope();
        Ok(instructions)
    }

    fn visit_function_expr(
        &mut self,
        f: &FunctionDeclaration<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = vec![Instruction::Op(Opcode::Closure)];

        let params = f.arguments.len();
        let statements = f.statements.clone(); // TODO: somehow avoid this clone

        let mut scope = ScopeGuard::new();
        scope.enter_scope();
        for argument in &f.arguments {
            scope.push_local(Local::new(argument, 0));
        }

        let mut frame = unsafe {
            Self::with_scopeguard(
                statements,
                scope,
                self.agent.as_mut().map(MaybeOwned::as_borrowed),
                // SAFETY: self is never null
                Some(NonNull::new_unchecked(self as *mut _)),
                false,
            )
            .compile_frame()
        }?;

        if frame.instructions.len() == 0 {
            frame.instructions.push(Instruction::Op(Opcode::Constant));
            frame
                .instructions
                .push(Instruction::Operand(Constant::JsValue(Value::new(
                    ValueKind::Undefined,
                ))));
            frame.instructions.push(Instruction::Op(Opcode::Return));
        } else if let Some(Instruction::Op(op)) = frame.instructions.last() {
            if !op.eq(&Opcode::Return) {
                frame.instructions.push(Instruction::Op(Opcode::Constant));
                frame
                    .instructions
                    .push(Instruction::Operand(Constant::JsValue(Value::new(
                        ValueKind::Undefined,
                    ))));
                frame.instructions.push(Instruction::Op(Opcode::Return));
            }
        }

        let mut func = UserFunction::new(
            frame.instructions,
            params as u32,
            FunctionType::Function,
            frame.upvalues.len() as u32,
            Constructor::Any,
        );
        if let Some(name) = f.name {
            func.name = Some(std::str::from_utf8(name).unwrap().to_owned());
        }
        instructions.push(Instruction::Operand(Constant::JsValue(func.into())));

        for upvalue in frame.upvalues.into_iter(IteratorOrder::BottomToTop) {
            if upvalue.local {
                instructions.push(Instruction::Op(Opcode::UpvalueLocal));
            } else {
                instructions.push(Instruction::Op(Opcode::UpvalueNonLocal));
            }
            instructions.push(Instruction::Operand(Constant::Index(upvalue.idx)));
        }
        Ok(instructions)
    }

    fn visit_function_declaration(
        &mut self,
        f: &FunctionDeclaration<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.visit_function_expr(f)?;

        if self.scope.is_global() {
            instructions.push(Instruction::Op(Opcode::Constant));
            instructions.push(Instruction::Operand(Constant::Identifier(
                std::str::from_utf8(f.name.unwrap()).unwrap().to_owned(),
            )));
            instructions.push(Instruction::Op(Opcode::SetGlobal));
        } else {
            let stack_idx = self
                .scope
                .push_local(Local::new(f.name.unwrap(), self.scope.depth));
            instructions.push(Instruction::Op(Opcode::Constant));
            instructions.push(Instruction::Operand(Constant::Index(stack_idx)));
            instructions.push(Instruction::Op(Opcode::SetLocal));
        }

        Ok(instructions)
    }

    fn visit_assignment_expression(
        &mut self,
        e: &AssignmentExpr<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&e.left)?;
        instructions.extend(self.accept_expr(&e.right)?);
        instructions.push(Instruction::Op(e.operator.into()));

        Ok(instructions)
    }

    fn visit_expression_statement(
        &mut self,
        e: &Expr<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(e)?;
        instructions.push(Instruction::Op(Opcode::Pop));
        Ok(instructions)
    }

    fn visit_function_call(
        &mut self,
        c: &FunctionCall<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&c.target)?;

        let argument_len = c.arguments.len();

        for argument in &c.arguments {
            instructions.extend(self.accept_expr(argument)?);
        }

        instructions.push(Instruction::Op(Opcode::Constant));
        instructions.push(Instruction::Operand(Constant::Index(argument_len)));

        if c.constructor_call {
            instructions.push(Instruction::Op(Opcode::ConstructorCall));
        } else {
            instructions.push(Instruction::Op(Opcode::FunctionCall));
        }

        Ok(instructions)
    }

    fn visit_return_statement(
        &mut self,
        s: &ReturnStatement<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&s.0)?;
        instructions.push(Instruction::Op(Opcode::Return));
        Ok(instructions)
    }

    fn visit_conditional_expr(
        &mut self,
        c: &ConditionalExpr<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&c.condition)?;

        instructions.push(Instruction::Op(Opcode::Constant));
        let then_jmp_idx = instructions.len();
        instructions.push(Instruction::Op(Opcode::Nop));

        instructions.push(Instruction::Op(Opcode::ShortJmpIfFalse));
        instructions.push(Instruction::Op(Opcode::Pop));
        let then_instructions = self.accept_expr(&c.then)?;
        let then_instruction_count = then_instructions.len();
        instructions.extend(then_instructions);
        instructions[then_jmp_idx] =
            Instruction::Operand(Constant::Index(then_instruction_count + 3));

        instructions.push(Instruction::Op(Opcode::Constant));
        let else_jmp_idx = instructions.len();
        instructions.push(Instruction::Op(Opcode::Nop));
        instructions.push(Instruction::Op(Opcode::ShortJmp));

        let else_instructions = self.accept_expr(&c.el)?;
        let else_instruction_count = else_instructions.len();
        instructions[else_jmp_idx] = Instruction::Operand(Constant::Index(else_instruction_count));
        instructions.extend(else_instructions);

        Ok(instructions)
    }

    fn visit_property_access_expr(
        &mut self,
        e: &PropertyAccessExpr<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&e.target)?;

        if e.computed {
            let property = self.accept_expr(&e.property)?;
            instructions.extend(property);
            instructions.push(Instruction::Op(Opcode::ComputedPropertyAccess));
        } else {
            let ident: &[u8] = if let Expr::Literal(lit) = &*e.property {
                match lit {
                    LiteralExpr::Identifier(ident) => ident,
                    _ => todo!(),
                }
            } else {
                todo!()
            };

            instructions.push(Instruction::Op(Opcode::Constant));
            instructions.push(Instruction::Operand(Constant::Identifier(
                std::str::from_utf8(ident).unwrap().to_owned(),
            )));

            instructions.push(Instruction::Op(Opcode::StaticPropertyAccess));
        }

        Ok(instructions)
    }

    fn visit_sequence_expr(&mut self, s: &Seq<'a>) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(&s.0)?;
        instructions.push(Instruction::Op(Opcode::Pop));

        let rhs = self.accept_expr(&s.1)?;
        instructions.extend(rhs);

        Ok(instructions)
    }

    fn visit_postfix_expr(
        &mut self,
        p: &Postfix<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut target = self.accept_expr(&p.1)?;
        target.push(Instruction::Op(p.0.into()));
        Ok(target)
    }

    fn visit_array_literal(
        &mut self,
        a: &ArrayLiteral<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let element_count = a.len();
        let mut instructions = Vec::new();
        for expr in a.iter().rev() {
            instructions.extend(self.accept_expr(expr)?);
        }
        instructions.push(Instruction::Op(Opcode::Constant));
        instructions.push(Instruction::Operand(Constant::Index(element_count)));

        instructions.push(Instruction::Op(Opcode::ArrayLiteral));
        Ok(instructions)
    }

    fn visit_object_literal(
        &mut self,
        o: &ObjectLiteral<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        let property_count = o.len();
        let mut instructions = Vec::new();

        // First we emit instructions for all object values
        for (_, value) in o.iter() {
            instructions.extend(self.accept_expr(value)?);
        }

        instructions.push(Instruction::Op(Opcode::Constant));
        instructions.push(Instruction::Operand(Constant::Index(property_count)));
        instructions.push(Instruction::Op(Opcode::ObjectLiteral));

        // ...And then we emit instructions for keys, because it shouldn't try to evaluate them at runtime
        for (key, _) in o.iter() {
            instructions.push(Instruction::Operand(Constant::Identifier(
                String::from_utf8_lossy(key).to_string(),
            )));
        }

        Ok(instructions)
    }

    fn visit_try_catch(&mut self, t: &TryCatch<'a>) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = vec![Instruction::Op(Opcode::Try), Instruction::Op(Opcode::Nop)];

        if t.catch.ident.is_some() {
            instructions.push(Instruction::Op(Opcode::SetLocal));
            instructions.push(Instruction::Op(Opcode::Nop));
        } else {
            instructions.push(Instruction::Op(Opcode::SetLocalNoValue));
        };

        let prefix_instructions = instructions.len();

        instructions.extend(self.accept(&t.try_)?);

        instructions.push(Instruction::Op(Opcode::PopUnwindHandler));
        instructions.push(Instruction::Op(Opcode::Constant));
        let thing_idx = instructions.len();
        instructions.push(Instruction::Op(Opcode::Nop));
        instructions.push(Instruction::Op(Opcode::ShortJmp));

        self.scope.enter_scope();

        if let Some(ident) = t.catch.ident {
            let stack_idx = self.scope.push_local(Local::new(ident, self.scope.depth));

            instructions[3] = Instruction::Operand(Constant::Index(stack_idx));
        }

        let catch = self.accept(&t.catch.body)?;
        self.scope.leave_scope();

        instructions[thing_idx] = Instruction::Operand(Constant::Index(catch.len()));

        let catch_jmp_idx = instructions.len();
        instructions.extend(catch);

        // ...add catch jump index
        instructions[1] = Instruction::Operand(Constant::Index(
            catch_jmp_idx - prefix_instructions, /* we skipped the first 4 instructions at this point in vm, so we subtract 2 */
        ));

        Ok(instructions)
    }

    fn visit_throw(&mut self, e: &Expr<'a>) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = self.accept_expr(e)?;
        instructions.push(Instruction::Op(Opcode::Throw));
        Ok(instructions)
    }

    fn visit_for_loop(&mut self, f: &ForLoop<'a>) -> Result<Vec<Instruction>, CompileError<'a>> {
        let mut instructions = vec![
            Instruction::Op(Opcode::LoopStart),
            Instruction::Op(Opcode::Nop),
            Instruction::Op(Opcode::Nop),
        ];
        self.scope.enter_scope();

        if let Some(initializer) = &f.init {
            instructions.extend(self.accept(initializer)?)
        }

        let begin_condition_idx = instructions.len();
        instructions[1] = Instruction::Operand(Constant::Index(begin_condition_idx - 3));

        if let Some(condition) = &f.condition {
            instructions.extend(self.accept_expr(condition)?);
        } else {
            instructions.extend(self.accept_expr(&Expr::bool_literal(true))?);
        };

        instructions.push(Instruction::Op(Opcode::Constant));
        let end_of_loop_jmp = instructions.len();
        instructions.push(Instruction::Op(Opcode::Nop));
        instructions.push(Instruction::Op(Opcode::ShortJmpIfFalse));

        instructions.push(Instruction::Op(Opcode::Pop));

        instructions.push(Instruction::Op(Opcode::Constant));
        let body_jmp = instructions.len();
        instructions.push(Instruction::Op(Opcode::Nop));
        instructions.push(Instruction::Op(Opcode::ShortJmp));

        let finalizer_idx = instructions.len();
        if let Some(finalizer) = &f.finalizer {
            instructions.extend(self.accept_expr(finalizer)?);
            instructions.push(Instruction::Op(Opcode::Pop));
        }

        instructions.push(Instruction::Op(Opcode::Constant));
        let condition_back_jmp = instructions.len();
        instructions.push(Instruction::Op(Opcode::Nop));
        instructions.push(Instruction::Op(Opcode::BackJmp));

        let begin_body = instructions.len();
        instructions.extend(self.accept(&f.body)?);

        instructions.push(Instruction::Op(Opcode::Constant));
        instructions.push(Instruction::Operand(Constant::Index(
            instructions.len() - finalizer_idx + 2,
        )));
        instructions.push(Instruction::Op(Opcode::BackJmp));

        instructions[end_of_loop_jmp] =
            Instruction::Operand(Constant::Index(instructions.len() - (end_of_loop_jmp + 2)));
        instructions[body_jmp] = Instruction::Operand(Constant::Index(begin_body - (body_jmp + 2)));
        instructions[condition_back_jmp] = Instruction::Operand(Constant::Index(
            condition_back_jmp - begin_condition_idx + 2,
        ));

        instructions.push(Instruction::Op(Opcode::Pop));

        instructions[2] = Instruction::Operand(Constant::Index(instructions.len() - 3));

        instructions.push(Instruction::Op(Opcode::LoopEnd));

        self.scope.leave_scope();

        Ok(instructions)
    }

    fn visit_break(&mut self) -> Result<Vec<Instruction>, CompileError<'a>> {
        Ok(vec![Instruction::Op(Opcode::Break)])
    }

    fn visit_continue(&mut self) -> Result<Vec<Instruction>, CompileError<'a>> {
        Ok(vec![Instruction::Op(Opcode::Continue)])
    }

    fn visit_import_statement(
        &mut self,
        i: &ImportKind<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        if let ImportKind::Dynamic(_) = i {
            return Err(CompileError::NotImplemented(
                "Dynamic imports are not implemented yet",
            ));
        }

        // todo: don't unwrap and handle dynamic imports
        let module_name = i.get_module_target().unwrap();

        let mut module_instructions = if let Some(agent) = &mut self.agent {
            let agent = unsafe { agent.as_mut() };

            match agent.import(module_name) {
                Some(ImportResult::Bytecode(code)) => code,
                Some(ImportResult::Value(value)) => vec![
                    Instruction::Op(Opcode::Constant),
                    Instruction::Operand(Constant::JsValue(value)),
                    Instruction::Op(Opcode::ExportDefault),
                    Instruction::Op(Opcode::ReturnModule),
                ],
                _ => return Err(CompileError::NativeImportFailed),
            }
        } else {
            return Err(CompileError::ImportDisabled);
        };

        // We only want to insert a ReturnModule opcode if it's not there already
        let has_module_return = match module_instructions.last() {
            Some(last) => matches!(last, Instruction::Op(Opcode::ReturnModule)),
            None => false,
        };

        if !has_module_return {
            module_instructions.push(Instruction::Op(Opcode::ReturnModule));
        }

        let mut instructions: Vec<Instruction> = vec![Instruction::Op(Opcode::EvaluateModule)];

        let module = Module::new(module_instructions);
        instructions.push(Instruction::Operand(Constant::JsValue(module.into())));

        let instructions = self.compile_variable_declaration(
            &VariableDeclaration::new(
                i.get_specifier().and_then(SpecifierKind::as_ident).unwrap(),
                VariableDeclarationKind::Var,
                None,
            ),
            instructions,
        )?;

        Ok(instructions)
    }

    fn visit_export_statement(
        &mut self,
        e: &ExportKind<'a>,
    ) -> Result<Vec<Instruction>, CompileError<'a>> {
        match e {
            ExportKind::Default(expr) => {
                let mut instructions = self.accept_expr(expr)?;
                instructions.push(Instruction::Op(Opcode::ExportDefault));
                Ok(instructions)
            }
            _ => Err(CompileError::NotImplemented(
                "Only default exports are currently supported",
            )),
        }
    }
}
