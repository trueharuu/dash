use crate::parser::expr::ArrayLiteral;
use crate::parser::expr::AssignmentExpr;
use crate::parser::expr::BinaryExpr;
use crate::parser::expr::ConditionalExpr;
use crate::parser::expr::Expr;
use crate::parser::expr::FunctionCall;
use crate::parser::expr::GroupingExpr;
use crate::parser::expr::LiteralExpr;
use crate::parser::expr::ObjectLiteral;
use crate::parser::expr::Postfix;
use crate::parser::expr::Prefix;
use crate::parser::expr::PropertyAccessExpr;
use crate::parser::expr::Seq;
use crate::parser::expr::UnaryExpr;
use crate::parser::statement::BlockStatement;
use crate::parser::statement::Class;
use crate::parser::statement::DoWhileLoop;
use crate::parser::statement::ExportKind;
use crate::parser::statement::ForInLoop;
use crate::parser::statement::ForLoop;
use crate::parser::statement::ForOfLoop;
use crate::parser::statement::FunctionDeclaration;
use crate::parser::statement::IfStatement;
use crate::parser::statement::ImportKind;
use crate::parser::statement::Loop;
use crate::parser::statement::ReturnStatement;
use crate::parser::statement::Statement;
use crate::parser::statement::SwitchStatement;
use crate::parser::statement::TryCatch;
use crate::parser::statement::VariableDeclarations;
use crate::parser::statement::WhileLoop;

pub trait VisitorExt<'a>: Visitor<'a, ()> {
    fn accept(&mut self, e: Statement<'a>);
    fn accept_expr(&mut self, e: Statement<'a>);
}

/// A visitor trait that helps walking an AST
pub trait Visitor<'a, V> {
    /// Accepts a parsed statement
    fn accept(&mut self, e: Statement<'a>) -> V;

    /// Accepts a parsed expression
    fn accept_expr(&mut self, e: Expr<'a>) -> V;

    /// Visits an expression statement
    fn visit_expression_statement(&mut self, e: Expr<'a>) -> V;

    /// Visits a binary expression
    fn visit_binary_expression(&mut self, e: BinaryExpr<'a>) -> V;

    /// Visits a grouping expression
    fn visit_grouping_expression(&mut self, e: GroupingExpr<'a>) -> V;

    /// Visits a literal expression
    fn visit_literal_expression(&mut self, e: LiteralExpr<'a>) -> V;

    /// Visits an identifier
    fn visit_identifier_expression(&mut self, i: &str) -> V;

    /// Visits an unary expression
    fn visit_unary_expression(&mut self, e: UnaryExpr<'a>) -> V;

    /// Visits a variable declaration
    fn visit_variable_declaration(&mut self, v: VariableDeclarations<'a>) -> V;

    /// Visits an if statement
    fn visit_if_statement(&mut self, i: IfStatement<'a>) -> V;

    /// Visits a block statement
    fn visit_block_statement(&mut self, b: BlockStatement<'a>) -> V;

    /// Visits a function declaration
    fn visit_function_declaration(&mut self, f: FunctionDeclaration<'a>) -> V;

    /// Visits a while loop
    fn visit_while_loop(&mut self, l: WhileLoop<'a>) -> V;

    /// Visits a do while loop
    fn visit_do_while_loop(&mut self, d: DoWhileLoop<'a>) -> V;

    /// Visits an assignment expression
    fn visit_assignment_expression(&mut self, e: AssignmentExpr<'a>) -> V;

    /// Visits a function call
    fn visit_function_call(&mut self, c: FunctionCall<'a>) -> V;

    /// Visits a return statement
    fn visit_return_statement(&mut self, s: ReturnStatement<'a>) -> V;

    /// Visits a conditional expression
    fn visit_conditional_expr(&mut self, c: ConditionalExpr<'a>) -> V;

    /// Visits a property access expression
    ///
    /// This includes both computed access and static access
    fn visit_property_access_expr(&mut self, e: PropertyAccessExpr<'a>, preserve_this: bool) -> V;

    /// Visits a sequence expression
    fn visit_sequence_expr(&mut self, s: Seq<'a>) -> V;

    /// Visits any prefix expression
    fn visit_prefix_expr(&mut self, p: Prefix<'a>) -> V;

    /// Visits any postfix expression
    fn visit_postfix_expr(&mut self, p: Postfix<'a>) -> V;

    /// Visits a function expression
    fn visit_function_expr(&mut self, f: FunctionDeclaration<'a>) -> V;

    /// Visits an array literal
    fn visit_array_literal(&mut self, a: ArrayLiteral<'a>) -> V;

    /// Visits an object literal
    fn visit_object_literal(&mut self, o: ObjectLiteral<'a>) -> V;

    /// Visits a try catch statement
    fn visit_try_catch(&mut self, t: TryCatch<'a>) -> V;

    /// Visits a throw statement
    fn visit_throw(&mut self, e: Expr<'a>) -> V;

    /// Visits a for loop
    fn visit_for_loop(&mut self, f: ForLoop<'a>) -> V;

    /// Visits a for..of loop
    fn visit_for_of_loop(&mut self, f: ForOfLoop<'a>) -> V;

    /// Visits a for..in loop
    fn visit_for_in_loop(&mut self, f: ForInLoop<'a>) -> V;

    /// Visits an import statement
    fn visit_import_statement(&mut self, i: ImportKind<'a>) -> V;

    /// Visits an export statement
    fn visit_export_statement(&mut self, e: ExportKind<'a>) -> V;

    /// Visits an empty statement
    fn visit_empty_statement(&mut self) -> V;

    /// Visits a break statement
    fn visit_break(&mut self) -> V;

    /// Visits a continue statement
    fn visit_continue(&mut self) -> V;

    /// Visits a debugger statement
    fn visit_debugger(&mut self) -> V;

    /// Visits an empty expression
    fn visit_empty_expr(&mut self) -> V;

    /// Visits a class declaration
    fn visit_class_declaration(&mut self, c: Class<'a>) -> V;

    /// Visits a switch statement
    fn visit_switch_statement(&mut self, s: SwitchStatement<'a>) -> V;
}

pub fn accept_default<'a, T, V: Visitor<'a, T>>(this: &mut V, s: Statement<'a>) -> T {
    match s {
        Statement::Expression(e) => this.visit_expression_statement(e),
        Statement::Variable(v) => this.visit_variable_declaration(v),
        Statement::If(i) => this.visit_if_statement(i),
        Statement::Block(b) => this.visit_block_statement(b),
        Statement::Function(f) => this.visit_function_declaration(f),
        Statement::Loop(Loop::For(f)) => this.visit_for_loop(f),
        Statement::Loop(Loop::While(w)) => this.visit_while_loop(w),
        Statement::Loop(Loop::ForOf(f)) => this.visit_for_of_loop(f),
        Statement::Loop(Loop::ForIn(f)) => this.visit_for_in_loop(f),
        Statement::Loop(Loop::DoWhile(d)) => this.visit_do_while_loop(d),
        Statement::Return(r) => this.visit_return_statement(r),
        Statement::Try(t) => this.visit_try_catch(t),
        Statement::Throw(t) => this.visit_throw(t),
        Statement::Import(i) => this.visit_import_statement(i),
        Statement::Export(e) => this.visit_export_statement(e),
        Statement::Class(c) => this.visit_class_declaration(c),
        Statement::Continue => this.visit_continue(),
        Statement::Break => this.visit_break(),
        Statement::Debugger => this.visit_debugger(),
        Statement::Empty => this.visit_empty_statement(),
        Statement::Switch(s) => this.visit_switch_statement(s),
    }
}

pub fn accept_expr_default<'a, T, V: Visitor<'a, T>, F>(this: &mut V, e: Expr<'a>, on_empty: F) -> T
where
    F: FnOnce(&mut V) -> T,
{
    match e {
        Expr::Binary(e) => this.visit_binary_expression(e),
        Expr::Assignment(e) => this.visit_assignment_expression(e),
        Expr::Grouping(e) => this.visit_grouping_expression(e),
        Expr::Literal(LiteralExpr::Identifier(i)) => this.visit_identifier_expression(&i),
        Expr::Literal(l) => this.visit_literal_expression(l),
        Expr::Unary(e) => this.visit_unary_expression(e),
        Expr::Call(e) => this.visit_function_call(e),
        Expr::Conditional(e) => this.visit_conditional_expr(e),
        Expr::PropertyAccess(e) => this.visit_property_access_expr(e, false),
        Expr::Sequence(e) => this.visit_sequence_expr(e),
        Expr::Postfix(e) => this.visit_postfix_expr(e),
        Expr::Prefix(e) => this.visit_prefix_expr(e),
        Expr::Function(e) => this.visit_function_expr(e),
        Expr::Array(e) => this.visit_array_literal(e),
        Expr::Object(e) => this.visit_object_literal(e),
        Expr::Compiled(..) => on_empty(this),
        Expr::Empty => this.visit_empty_expr(),
    }
}
