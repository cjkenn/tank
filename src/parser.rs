use lexer::Lexer;
use token::Token;
use token::TokenType;
use ast::Ast;
use ast::AstType;

pub struct Parser {
    lexer: Lexer,
    curr_val: String,
    curr_type: TokenType
}

impl Parser {
    pub fn new(template: String) -> Parser {
        let mut l = Lexer::new(template);
        l.lex();
        let tok = l.curr_tok
            .take()
            .unwrap_or(Token::new(TokenType::Eof));

        let tv = tok.val;
        let tt = tok.tok_type;

        Parser {
            lexer: l,
            curr_val: tv,
            curr_type: tt,
        }
    }

    pub fn parse(&mut self) -> Ast {
        if self.curr_type == TokenType::Eof {
            panic!("tank: End of input reached, nothing to parse!");
        }

        let mut root_ast = Ast::new(AstType::Template);
        root_ast.children.push(self.element());

        println!("{:?}", root_ast.children);

        root_ast
    }

    // TODO: error checking better here
    fn element(&mut self) -> Box<Ast> {
        let mut el_ast = Ast::new(AstType::Element);
        match self.curr_type {
            TokenType::Ident => {
                match self.curr_val.as_ref() {
                    "if" => {
                        // Consume "if"
                        self.get_next_tok();
                        el_ast = Ast::new(AstType::IfExpr);
                        el_ast.children.push(self.expr());

                        // Consume "{"
                        self.get_next_tok();

                        el_ast.children.push(self.element());

                        // Consume "}"
                        self.get_next_tok();
                        el_ast.children.push(self.element());
                    },
                    "for" => {
                        // Consume "for"
                        self.get_next_tok();
                        el_ast = Ast::new(AstType::ForExpr);
                        el_ast.children.push(self.term());

                        if self.curr_val != "in" {
                            panic!("tank: Parse error - Expected 'in' at for loop");
                        } else {
                            self.get_next_tok();
                        }

                        el_ast.children.push(self.term());
                        el_ast.children.push(self.element());
                    },
                    "let" => {
                        // Consume "let"
                        self.get_next_tok();

                        el_ast.children.push(self.expr());
                        el_ast.children.push(self.element());
                    },
                    _ => {
                        el_ast.children.push(self.term());

                        if self.curr_type == TokenType::LeftParen {
                            el_ast.children.push(self.attr_list());
                        }

                        el_ast.children.push(self.element());
                    }
                };
            },
            TokenType::LeftBrace => {
                //  Consume "{"
                self.get_next_tok();

                el_ast.children.push(self.element());

                // Consume "}"
                self.get_next_tok();
            },
            _ => {
                return Box::new(el_ast);
            }
        }

        Box::new(el_ast)
    }

    fn attr_list(&mut self) -> Box<Ast> {
        let mut attr_ast = Ast::new(AstType::AttrList);

        if self.curr_type == TokenType::LeftParen {
            self.get_next_tok();
        } else {
            panic!("tank: Parse error - Expected '('")
        }

        while self.curr_type != TokenType::RightParen {
            attr_ast.children.push(self.term());

            if self.curr_type == TokenType::Colon {
                self.get_next_tok();
            } else {
                panic!("tank: Parse error - Expected ':'")
            }

            attr_ast.children.push(self.term());
        }

        if self.curr_type == TokenType::RightParen {
            self.get_next_tok();
        } else {
            panic!("tank: Parse error - Expected ')'");
        }

        if self.curr_type == TokenType::Arrow {
            self.get_next_tok();
        } else {
            panic!("tank: Parse error - Expected '->'");
        }

        Box::new(attr_ast)
    }

    fn expr(&mut self) -> Box<Ast> {
        // if self.curr_type != TokenType::Ident {
        //     return self.test();
        // }

        let expr_ast = self.test();

        // if expr_ast.ast_type == AstType::Ident && self.curr_type == TokenType::Colon {
        //     self.get_next_tok();
        //     let expr_ast_next = expr_ast;

        //     expr_ast = Box::new(Ast::new(AstType::AssignExpr));
        //     expr_ast.var_type = Some(self.curr_val.clone());

        //     self.get_next_tok();

        //     expr_ast.children.push(expr_ast_next);
        //     expr_ast.children.push(self.expr());
        // }

        expr_ast
    }

    fn test(&mut self) -> Box<Ast> {
        let mut test_ast = self.op();

        let curr_ast_type = match self.curr_type {
            TokenType::Gt => AstType::Gt,
            TokenType::Lt => AstType::Lt,
            TokenType::GtEquals => AstType::GtEquals,
            TokenType::LtEquals => AstType::LtEquals,
            TokenType::NotEquals => AstType::NotEquals,
            TokenType::EqualsEquals => AstType::EqualsEquals,
            TokenType::Colon => {
                self.get_next_tok();
                test_ast.var_type = Some(self.curr_val.clone());
                self.get_next_tok();

                AstType::AssignExpr
            },
            _ => test_ast.ast_type.clone()
        };

        let test_ast_next = test_ast;
        test_ast = Box::new(Ast::new(curr_ast_type));
        self.get_next_tok();

        test_ast.children.push(test_ast_next);
        test_ast.children.push(self.op());

        test_ast
    }

    fn op(&mut self) -> Box<Ast> {
        let mut op_ast = self.term();

        while self.curr_type == TokenType::Plus || self.curr_type == TokenType::Minus {
            let op_ast_next = op_ast;

            //TODO: Currently, only supporting plus and minus
            let curr_ast_type = match self.curr_type {
                TokenType::Plus => AstType::Plus,
                TokenType::Minus => AstType::Minus,
                _ => AstType::Empty
            };

            op_ast = Box::new(Ast::new(curr_ast_type));
            self.get_next_tok();
            op_ast.children.push(op_ast_next);
            op_ast.children.push(self.term());
        }

        op_ast
    }

    fn term(&mut self) -> Box<Ast> {
        let term_ast;

        match self.curr_type {
            TokenType::Ident => {
                term_ast = Box::new(Ast::new_with_val(AstType::Ident, self.curr_val.clone()));
                self.get_next_tok();
            },
            TokenType::Number => {
                term_ast = Box::new(Ast::new_with_val(AstType::Number, self.curr_val.clone()));
                self.get_next_tok();
            },
            _ => {
                term_ast = self.expr();
            }
        }

        term_ast
    }

    fn get_next_tok(&mut self) -> &mut Parser {
        self.lexer.lex();

        let tok = self.lexer.curr_tok
            .take()
            .unwrap_or(Token::new(TokenType::Eof));

        self.curr_val = tok.val;
        self.curr_type = tok.tok_type;

        self
    }
}
