use dash_middle::lexer::token::TokenType;
use dash_middle::lexer::token::VARIABLE_TYPES;
use dash_middle::parser::error::ErrorKind;
use dash_middle::parser::expr::Expr;
use dash_middle::parser::statement::BlockStatement;
use dash_middle::parser::statement::Catch;
use dash_middle::parser::statement::Class;
use dash_middle::parser::statement::ClassMember;
use dash_middle::parser::statement::ClassMemberKind;
use dash_middle::parser::statement::ClassProperty;
use dash_middle::parser::statement::DoWhileLoop;
use dash_middle::parser::statement::ExportKind;
use dash_middle::parser::statement::ForInLoop;
use dash_middle::parser::statement::ForLoop;
use dash_middle::parser::statement::ForOfLoop;
use dash_middle::parser::statement::FunctionDeclaration;
use dash_middle::parser::statement::FunctionKind;
use dash_middle::parser::statement::IfStatement;
use dash_middle::parser::statement::ImportKind;
use dash_middle::parser::statement::Loop;
use dash_middle::parser::statement::Parameter;
use dash_middle::parser::statement::ReturnStatement;
use dash_middle::parser::statement::SpecifierKind;
use dash_middle::parser::statement::Statement;
use dash_middle::parser::statement::SwitchCase;
use dash_middle::parser::statement::SwitchStatement;
use dash_middle::parser::statement::TryCatch;
use dash_middle::parser::statement::VariableBinding;
use dash_middle::parser::statement::VariableDeclaration;
use dash_middle::parser::statement::VariableDeclarationKind;
use dash_middle::parser::statement::VariableDeclarationName;
use dash_middle::parser::statement::VariableDeclarations;
use dash_middle::parser::statement::WhileLoop;
use dash_middle::parser::types::TypeSegment;

use crate::expr::ExpressionParser;
use crate::must_borrow_lexeme;
use crate::types::TypeParser;
use crate::Parser;

type ParameterList<'a> = Option<Vec<(Parameter<'a>, Option<Expr<'a>>, Option<TypeSegment<'a>>)>>;

pub trait StatementParser<'a> {
    fn parse_statement(&mut self) -> Option<Statement<'a>>;
    fn parse_class(&mut self) -> Option<Class<'a>>;
    fn parse_export(&mut self) -> Option<ExportKind<'a>>;
    fn parse_import(&mut self) -> Option<ImportKind<'a>>;
    fn parse_throw(&mut self) -> Option<Expr<'a>>;
    fn parse_try(&mut self) -> Option<TryCatch<'a>>;
    fn parse_return(&mut self) -> Option<ReturnStatement<'a>>;
    fn parse_for_loop(&mut self) -> Option<Loop<'a>>;
    fn parse_while_loop(&mut self) -> Option<Loop<'a>>;
    fn parse_do_while_loop(&mut self) -> Option<Loop<'a>>;
    fn parse_block(&mut self) -> Option<BlockStatement<'a>>;
    fn parse_variable(&mut self) -> Option<VariableDeclarations<'a>>;
    fn parse_variable_binding_with_kind(&mut self, kind: VariableDeclarationKind) -> Option<VariableBinding<'a>>;
    /// Parses a variable binding, i.e. `let x`
    fn parse_variable_binding(&mut self) -> Option<VariableBinding<'a>>;
    /// Parses the definition segment of a variable declaration statement, i.e. `= 5`
    fn parse_variable_definition(&mut self) -> Option<Expr<'a>>;
    fn parse_if(&mut self, parse_else: bool) -> Option<IfStatement<'a>>;
    fn parse_switch(&mut self) -> Option<SwitchStatement<'a>>;
    /// Parses a list of parameters (identifier, followed by optional type segment) delimited by comma,
    /// assuming that the ( has already been consumed
    fn parse_parameter_list(&mut self) -> ParameterList<'a>;
}

impl<'a> StatementParser<'a> for Parser<'a> {
    fn parse_statement(&mut self) -> Option<Statement<'a>> {
        self.error_sync = false;
        let stmt = match self.next()?.ty {
            TokenType::Let | TokenType::Const | TokenType::Var => self.parse_variable().map(Statement::Variable),
            TokenType::If => self.parse_if(true).map(Statement::If),
            TokenType::Function => self.parse_function(false).map(Statement::Function),
            TokenType::Async => {
                // async must be followed by function (todo: or async () => {})
                if !self.expect_and_skip(&[TokenType::Function], true) {
                    return None;
                }
                self.parse_function(true).map(Statement::Function)
            }
            TokenType::LeftBrace => self.parse_block().map(Statement::Block),
            TokenType::While => self.parse_while_loop().map(Statement::Loop),
            TokenType::Do => self.parse_do_while_loop().map(Statement::Loop),
            TokenType::Try => self.parse_try().map(Statement::Try),
            TokenType::Throw => self.parse_throw().map(Statement::Throw),
            TokenType::Return => self.parse_return().map(Statement::Return),
            TokenType::For => self.parse_for_loop().map(Statement::Loop),
            TokenType::Import => self.parse_import().map(Statement::Import),
            TokenType::Export => self.parse_export().map(Statement::Export),
            TokenType::Class => self.parse_class().map(Statement::Class),
            TokenType::Switch => self.parse_switch().map(Statement::Switch),
            TokenType::Continue => Some(Statement::Continue),
            TokenType::Break => Some(Statement::Break),
            TokenType::Debugger => Some(Statement::Debugger),
            _ => {
                // We've skipped the current character because of the statement cases that skip the current token
                // So we go back, as the skipped token belongs to this expression
                self.advance_back();
                Some(Statement::Expression(self.parse_expression()?))
            }
        };

        self.expect_and_skip(&[TokenType::Semicolon], false);

        stmt
    }

    fn parse_class(&mut self) -> Option<Class<'a>> {
        let name = if self.expect_identifier_and_skip(false) {
            let prev = self.previous()?;
            Some(must_borrow_lexeme!(self, prev)?)
        } else {
            None
        };

        let extends = if self.expect_and_skip(&[TokenType::Extends], false) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_and_skip(&[TokenType::LeftBrace], true);

        let mut members = Vec::new();

        // Start parsing class members
        while !self.expect_and_skip(&[TokenType::RightBrace], false) {
            let is_static = self.expect_and_skip(&[TokenType::Static], false);
            let is_private = self.expect_and_skip(&[TokenType::Hash], false);

            let name = self.next_identifier()?;

            let is_method = self.expect_and_skip(&[TokenType::LeftParen], false);

            if is_method {
                let arguments = self.parse_parameter_list()?;
                let body = self.parse_statement()?;

                let func_id = self.function_counter.advance();
                let func = FunctionDeclaration::new(
                    Some(name),
                    func_id,
                    arguments,
                    vec![body],
                    FunctionKind::Function,
                    false,
                );

                members.push(ClassMember {
                    private: is_private,
                    static_: is_static,
                    kind: ClassMemberKind::Method(func),
                });
            } else {
                let kind = self.next()?.ty;

                let value = match kind {
                    TokenType::Assignment => Some(self.parse_expression()?),
                    TokenType::Semicolon => None,
                    _ => {
                        // We don't know what this token is, so we assume the user left out the semicolon and meant to declare a property
                        // For this reason we need to go back so we don't throw away the token we just read
                        self.advance_back();
                        None
                    }
                };

                self.expect_and_skip(&[TokenType::Semicolon], false);

                members.push(ClassMember {
                    private: is_private,
                    static_: is_static,
                    kind: ClassMemberKind::Property(ClassProperty { name, value }),
                });
            };
        }

        Some(Class { name, extends, members })
    }

    fn parse_export(&mut self) -> Option<ExportKind<'a>> {
        let is_named = self.expect_and_skip(&[TokenType::LeftBrace], false);

        if is_named {
            let mut names = Vec::new();
            while !self.expect_and_skip(&[TokenType::RightBrace], false) {
                let name = self.next_identifier()?;
                names.push(name);
                self.expect_and_skip(&[TokenType::Comma], false);
            }
            return Some(ExportKind::Named(names));
        }

        let current = self.current()?;

        if current.ty.is_variable() {
            self.advance();
            let variables = self.parse_variable()?;

            return Some(ExportKind::NamedVar(variables));
        }

        // We emit an error because this is the last possible way to create
        // an export statement
        if self.expect_and_skip(&[TokenType::Default], true) {
            let expr = self.parse_expression()?;
            return Some(ExportKind::Default(expr));
        }

        None
    }

    fn parse_import(&mut self) -> Option<ImportKind<'a>> {
        // `import` followed by ( is considered a dynamic import
        let is_dynamic = self.expect_and_skip(&[TokenType::LeftParen], false);
        if is_dynamic {
            let specifier = self.parse_expression()?;
            self.expect_and_skip(&[TokenType::RightParen], true);
            return Some(ImportKind::Dynamic(specifier));
        }

        // `import` followed by a `*` imports all exported values
        let is_import_all = self.expect_and_skip(&[TokenType::Star], false);
        if is_import_all {
            self.expect_identifier_and_skip(true);
            // TODO: enforce identifier be == b"as"
            let ident = self.next_identifier()?;
            self.expect_identifier_and_skip(true);
            // TODO: enforce identifier be == b"from"
            let specifier = self.next()?.full.clone();
            return Some(ImportKind::AllAs(SpecifierKind::Ident(ident), specifier));
        }

        // `import` followed by an identifier is considered a default import
        if let Some(default_import_ident) = self.next_identifier() {
            self.expect_identifier_and_skip(true);
            // TODO: enforce identifier be == b"from"
            let specifier = self.next()?.full.clone();
            return Some(ImportKind::DefaultAs(
                SpecifierKind::Ident(default_import_ident),
                specifier,
            ));
        }

        None
    }

    fn parse_throw(&mut self) -> Option<Expr<'a>> {
        self.parse_expression()
    }

    fn parse_try(&mut self) -> Option<TryCatch<'a>> {
        let try_ = self.parse_statement()?;

        self.expect_and_skip(&[TokenType::Catch], true);

        let capture_ident = if self.expect_and_skip(&[TokenType::LeftParen], false) {
            let ident = self.next_identifier()?;
            self.expect_and_skip(&[TokenType::RightParen], true);
            Some(ident)
        } else {
            None
        };

        let catch = self.parse_statement()?;

        // TODO: finally

        Some(TryCatch::new(try_, Catch::new(catch, capture_ident), None))
    }

    fn parse_return(&mut self) -> Option<ReturnStatement<'a>> {
        let expr = self.parse_expression()?;
        Some(ReturnStatement(expr))
    }

    fn parse_for_loop(&mut self) -> Option<Loop<'a>> {
        self.expect_and_skip(&[TokenType::LeftParen], true);

        let init = if self.expect_and_skip(&[TokenType::Semicolon], false) {
            None
        } else {
            let is_binding = self.expect_and_skip(VARIABLE_TYPES, false);

            if is_binding {
                let binding = self.parse_variable_binding()?;
                let is_of_or_in = self.expect_and_skip(&[TokenType::Of, TokenType::In], false);

                if is_of_or_in {
                    let ty = self.previous()?.ty;
                    let expr = self.parse_expression()?;

                    self.expect_and_skip(&[TokenType::RightParen], true);

                    let body = Box::new(self.parse_statement()?);

                    return Some(match ty {
                        TokenType::In => Loop::ForIn(ForInLoop { binding, expr, body }),
                        TokenType::Of => Loop::ForOf(ForOfLoop { binding, expr, body }),
                        _ => unreachable!(),
                    });
                } else {
                    let value = self.parse_variable_definition();

                    self.expect_and_skip(&[TokenType::Semicolon], true);

                    Some(Statement::Variable(VariableDeclarations(vec![
                        VariableDeclaration::new(binding, value),
                    ])))
                }
            } else {
                let stmt = self.parse_statement();
                // The call to statement must have skipped a semicolon
                self.expect_previous(&[TokenType::Semicolon], true);
                stmt
            }
        };

        let cond = if self.expect_and_skip(&[TokenType::Semicolon], false) {
            None
        } else {
            let expr = self.parse_expression();
            self.expect_and_skip(&[TokenType::Semicolon], true);
            expr
        };

        let finalizer = if self.expect_and_skip(&[TokenType::RightParen], false) {
            None
        } else {
            let expr = self.parse_expression();
            self.expect_and_skip(&[TokenType::RightParen], true);
            expr
        };

        let body = self.parse_statement()?;

        Some(ForLoop::new(init, cond, finalizer, body).into())
    }

    fn parse_while_loop(&mut self) -> Option<Loop<'a>> {
        if !self.expect_and_skip(&[TokenType::LeftParen], true) {
            return None;
        }

        let condition = self.parse_expression()?;

        if !self.expect_and_skip(&[TokenType::RightParen], true) {
            return None;
        }

        let body = self.parse_statement()?;

        Some(WhileLoop::new(condition, body).into())
    }

    fn parse_do_while_loop(&mut self) -> Option<Loop<'a>> {
        let body = self.parse_statement()?;

        if !self.expect_and_skip(&[TokenType::While], true) {
            return None;
        }

        let condition = self.parse_expression()?;

        Some(DoWhileLoop::new(condition, body).into())
    }

    /// Parses a block. Assumes that the left brace `{` has already been consumed.
    fn parse_block(&mut self) -> Option<BlockStatement<'a>> {
        let mut stmts = Vec::new();
        while !self.expect_and_skip(&[TokenType::RightBrace], false) {
            if self.is_eof() {
                return None;
            }

            if let Some(stmt) = self.parse_statement() {
                stmts.push(stmt);
            }
        }
        Some(BlockStatement(stmts))
    }

    fn parse_variable(&mut self) -> Option<VariableDeclarations<'a>> {
        let mut decls = Vec::new();

        let initial_kind = {
            let binding = self.parse_variable_binding()?;
            let value = self.parse_variable_definition();
            let kind = binding.kind;
            decls.push(VariableDeclaration::new(binding, value));
            kind
        };

        while self.expect_and_skip(&[TokenType::Comma], false) {
            let binding = self.parse_variable_binding_with_kind(initial_kind)?;
            let value = self.parse_variable_definition();
            decls.push(VariableDeclaration::new(binding, value));
        }

        Some(VariableDeclarations(decls))
    }

    fn parse_if(&mut self, parse_else: bool) -> Option<IfStatement<'a>> {
        if !self.expect_and_skip(&[TokenType::LeftParen], true) {
            return None;
        }

        let condition = self.parse_expression()?;

        if !self.expect_and_skip(&[TokenType::RightParen], true) {
            return None;
        }

        let then = self.parse_statement()?;

        let mut branches = Vec::new();
        let mut el: Option<Box<Statement>> = None;

        if parse_else {
            while self.expect_and_skip(&[TokenType::Else], false) {
                let is_if = self.expect_and_skip(&[TokenType::If], false);

                if is_if {
                    let if_statement = self.parse_if(false)?;
                    branches.push(if_statement);
                } else {
                    el = Some(Box::new(self.parse_statement()?));
                    break;
                }
            }
        }

        Some(IfStatement::new(condition, then, branches, el))
    }

    fn parse_parameter_list(&mut self) -> ParameterList<'a> {
        let mut parameters = Vec::new();

        while !self.expect_and_skip(&[TokenType::RightParen], false) {
            let tok = self.next().cloned()?;

            let parameter = match tok.ty {
                TokenType::Dot => {
                    // Begin of spread operator
                    for _ in 0..2 {
                        self.expect_and_skip(&[TokenType::Dot], true);
                    }

                    let ident = self.next_identifier()?;

                    Parameter::Spread(ident)
                }
                TokenType::Comma => continue,
                other if other.is_identifier() => Parameter::Identifier(must_borrow_lexeme!(self, &tok)?),
                _ => {
                    self.create_error(ErrorKind::UnexpectedToken(tok.clone(), TokenType::Comma));
                    return None;
                }
            };

            // Parse type param
            let ty = if self.expect_and_skip(&[TokenType::Colon], false) {
                Some(self.parse_type_segment()?)
            } else {
                None
            };

            // Parse default value
            let default = if self.expect_and_skip(&[TokenType::Assignment], false) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            let is_spread = matches!(parameter, Parameter::Spread(..));

            parameters.push((parameter, default, ty));

            if is_spread {
                // Must be followed by )
                if !self.expect_and_skip(&[TokenType::RightParen], true) {
                    return None;
                }

                break;
            }
        }

        Some(parameters)
    }

    fn parse_variable_binding_with_kind(&mut self, kind: VariableDeclarationKind) -> Option<VariableBinding<'a>> {
        let name = if self.expect_and_skip(&[TokenType::LeftBrace], false) {
            // Object destructuring
            let mut fields = Vec::new();
            let mut rest = None;

            while !self.expect_and_skip(&[TokenType::RightBrace], false) {
                self.expect_and_skip(&[TokenType::Comma], false); // TODO: should be an error

                let cur = self.current()?.clone();
                match cur.ty {
                    TokenType::Dot => {
                        // Skip the dot
                        self.advance();
                        // Begin of rest operator, must be followed by two more dots
                        for _ in 0..2 {
                            self.expect_and_skip(&[TokenType::Dot], true);
                        }

                        let name = self.current()?.clone();
                        if name.ty.is_identifier() {
                            if rest.is_some() {
                                // Only allow one rest operator
                                self.create_error(ErrorKind::MultipleRestInDestructuring(name));
                                return None;
                            }

                            rest = Some(must_borrow_lexeme!(self, &name)?);
                            self.advance();
                        } else {
                            self.create_error(ErrorKind::UnexpectedToken(name, TokenType::Identifier));
                            return None;
                        }
                    }
                    other if other.is_identifier() => {
                        let name = must_borrow_lexeme!(self, &cur)?;
                        self.advance();

                        let alias = if self.expect_and_skip(&[TokenType::Colon], false) {
                            let alias = self.current()?.clone();
                            if alias.ty.is_identifier() {
                                self.advance();
                                Some(must_borrow_lexeme!(self, &alias)?)
                            } else {
                                self.create_error(ErrorKind::UnexpectedToken(alias, TokenType::Identifier));
                                return None;
                            }
                        } else {
                            None
                        };
                        fields.push((name, alias));
                    }
                    _ => {
                        self.create_error(ErrorKind::UnexpectedToken(cur, TokenType::Identifier));
                        return None;
                    }
                }
            }

            VariableDeclarationName::ObjectDestructuring { fields, rest }
        } else if self.expect_and_skip(&[TokenType::LeftSquareBrace], false) {
            // Array destructuring
            let mut fields = Vec::new();
            let mut rest = None;

            while !self.expect_and_skip(&[TokenType::RightSquareBrace], false) {
                self.expect_and_skip(&[TokenType::Comma], false); // TODO: should be an error

                let cur = self.current()?.clone();
                match cur.ty {
                    TokenType::Dot => {
                        // Skip the dot
                        self.advance();
                        // Begin of rest operator, must be followed by two more dots
                        for _ in 0..2 {
                            self.expect_and_skip(&[TokenType::Dot], true);
                        }

                        let name = self.current()?.clone();
                        if name.ty.is_identifier() {
                            if rest.is_some() {
                                // Only allow one rest operator
                                self.create_error(ErrorKind::MultipleRestInDestructuring(name));
                                return None;
                            }

                            rest = Some(must_borrow_lexeme!(self, &name)?);
                            self.advance();
                        } else {
                            self.create_error(ErrorKind::UnexpectedToken(name, TokenType::Identifier));
                            return None;
                        }
                    }
                    other if other.is_identifier() => {
                        let name = must_borrow_lexeme!(self, &cur)?;
                        self.advance();
                        fields.push(name);
                    }
                    _ => {
                        self.create_error(ErrorKind::UnexpectedToken(cur, TokenType::Identifier));
                        return None;
                    }
                }
            }

            VariableDeclarationName::ArrayDestructuring { fields, rest }
        } else {
            // Identifier
            let name = self.next_identifier()?;
            VariableDeclarationName::Identifier(name)
        };

        let ty = if self.expect_and_skip(&[TokenType::Colon], false) {
            Some(self.parse_type_segment()?)
        } else {
            None
        };

        Some(VariableBinding { kind, name, ty })
    }

    fn parse_variable_binding(&mut self) -> Option<VariableBinding<'a>> {
        let kind: VariableDeclarationKind = self.previous()?.ty.into();
        self.parse_variable_binding_with_kind(kind)
    }

    fn parse_variable_definition(&mut self) -> Option<Expr<'a>> {
        // If the next char is `=`, we assume this declaration has a value
        let has_value = self.expect_and_skip(&[TokenType::Assignment], false);

        if !has_value {
            return None;
        }

        self.parse_expression()
    }

    fn parse_switch(&mut self) -> Option<SwitchStatement<'a>> {
        self.expect_and_skip(&[TokenType::LeftParen], true);
        let value = self.parse_expression()?;
        self.expect_and_skip(&[TokenType::RightParen], true);

        self.expect_and_skip(&[TokenType::LeftBrace], true);

        let mut cases = Vec::new();
        let mut default = None;

        // Parse cases
        while !self.expect_and_skip(&[TokenType::RightBrace], false) {
            let cur = self.current()?.clone();
            self.next()?;

            match cur.ty {
                TokenType::Case => {
                    let value = self.parse_expression()?;
                    self.expect_and_skip(&[TokenType::Colon], true);

                    let mut body = Vec::new();
                    while !self.expect(&[TokenType::Case, TokenType::Default, TokenType::RightBrace]) {
                        body.push(self.parse_statement()?);
                    }

                    cases.push(SwitchCase { body, value });
                }
                TokenType::Default => {
                    self.expect_and_skip(&[TokenType::Colon], true);

                    let mut body = Vec::new();
                    while !self.expect(&[TokenType::Case, TokenType::Default, TokenType::RightBrace]) {
                        body.push(self.parse_statement()?);
                    }

                    if default.replace(body).is_some() {
                        self.create_error(ErrorKind::MultipleDefaultInSwitch(cur));
                        return None;
                    }
                }
                _ => {
                    self.create_error(ErrorKind::UnexpectedTokenMultiple(
                        cur,
                        &[TokenType::Case, TokenType::Default],
                    ));
                    return None;
                }
            }
        }

        Some(SwitchStatement {
            cases,
            default,
            expr: value,
        })
    }
}
