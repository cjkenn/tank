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

    fn element(&mut self) -> Box<Ast> {
        let mut el_ast = Ast::new(AstType::Element);

        match self.curr_type {
            TokenType::Ident => {
                match self.curr_val.as_ref() {
                    "if" => {
                        self.get_next_tok();
                        self.expr();
                        self.element();
                    },
                    "for" => {
                        self.get_next_tok();
                        self.ident();

                        if (self.curr_val != "in") {
                            panic!("tank: Parse error - Expected 'in' at for loop");
                        } else {
                            self.get_next_tok();
                        }

                        self.ident();
                        self.element();
                    },
                    "let" => {
                        self.get_next_tok();
                        self.expr();
                    },
                    _ => {
                        self.ident();
                        self.attr_list();
                        self.element();
                    }
                };
            },
            TokenType::LeftBrace => {
                self.get_next_tok();
                self.element();
                self.get_next_tok();
            },
            _ => {
                panic!("tank: Parse error - Unexpected token found");
            }
        }

        Box::new(el_ast)
    }

    // Expects the current token to be the identifier
    fn ident(&mut self) -> Box<Ast> {
        if self.curr_type != TokenType::Ident {
            panic!("tank: Parse error - Expected identifier");
        }

        let ident_ast = Ast::new_with_val(AstType::Ident, self.curr_val.clone());

        Box::new(ident_ast)
    }

    fn number(&mut self) -> Box<Ast> {
        if self.curr_type != TokenType::Number {
            panic!("tank: Parse error - Expected number");
        }

        let num_ast = Ast::new_with_val(AstType::Number, self.curr_val.clone());

        Box::new(num_ast)
    }

    fn el_content(&mut self) -> Box<Ast> {
        let content_ast = Ast::new_with_val(AstType::ElContent, self.curr_val.clone());

        Box::new(content_ast)
    }

    fn attr_list(&mut self) -> Box<Ast> {
        let mut attr_ast = Ast::new(AstType::AttrList);

        if self.curr_type == TokenType::LeftParen {
            self.get_next_tok();
        } else {
            panic!("tank: Parse error - Expected '('")
        }

        while self.curr_type != TokenType::RightParen {
            attr_ast.children.push(self.ident());

            self.get_next_tok();

            if self.curr_type == TokenType::Colon {
                self.get_next_tok();
            } else {
                panic!("tank: Parse error - Expected ':'")
            }

            attr_ast.children.push(self.ident());
            self.get_next_tok();
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

    // Expects current token to be "if" when called
    // TODO: support if true {...}
    fn if_expr(&mut self) -> Box<Ast> {
        // Advance to next tok
        self.get_next_tok();

        let mut if_ast = Ast::new(AstType::IfExpr);

        // Now in conditional statement
        match self.curr_type {
            TokenType::Ident => if_ast.children.push(self.ident()),
            TokenType::Number => if_ast.children.push(self.number()),
            _ => panic!("tank: Expected an identifier or number at start of if expression")
        };

        // Advance to operator
        self.get_next_tok();

        if !self.lexer.is_op(&self.curr_type) {
            panic!("tank: Expected an operator in if statement");
        }

        if_ast.val = self.curr_val.clone();

        // Advance to next id or number
        self.get_next_tok();

        match self.curr_type {
            TokenType::Ident => if_ast.children.push(self.ident()),
            TokenType::Number => if_ast.children.push(self.number()),
            _ => panic!("tank: Expected an identifier or number at end of if expression")
        };

        self.get_next_tok();

        if self.curr_type == TokenType::LeftBrace {
            self.get_next_tok();
        } else {
            panic!("tank: Expected '{'");
        }

        // New we're at the inner contents of the if
        if_ast.children.push(self.element());

        if self.curr_type == TokenType::RightBrace {
            self.get_next_tok();
        } else {
            panic!("tank: Expected '}'");
        }

        Box::new(if_ast)
    }

    // Expects current token to be "for"
    fn for_expr(&mut self) -> Box<Ast> {
        self.get_next_tok();

        let mut for_ast = Ast::new(AstType::ForExpr);

        match self.curr_type {
            TokenType::Ident => {
                for_ast.children.push(self.ident());

                // Advance to 'in' token
                self.get_next_tok();
            },
            _ => panic!("tank: Expected an id in for loop declaration")
        };

        match self.curr_val.as_ref() {
            "in" => self.get_next_tok(),
            _ => panic!("tank: Expected 'in' token in for loop declaration")
        };

        // Expect another ident here (the containing var)
        match self.curr_type {
            TokenType::Ident => {
                for_ast.val = self.curr_val.clone();
                self.get_next_tok();
            },
            _ => panic!("tank: Expected an id in for loop declaration")
        };

        // Expect a left brace here
        match self.curr_type {
            TokenType::LeftBrace => self.get_next_tok(),
            _ => panic!("tank: Expected '{'")
        };

        for_ast.children.push(self.element());

        // Expect a right brace here
        match self.curr_type {
            TokenType::RightBrace => self.get_next_tok(),
            _ => panic!("tank: Expected '}'")
        };

        Box::new(for_ast)
    }

    // Expects current token to be "let" when called.
    fn assign_expr(&mut self) -> Box<Ast> {
        // Advance to var name
        self.get_next_tok();

        if self.curr_type != TokenType::Ident {
            panic!("tank: Parse error - Expected an identifier");
        }

        let mut assign_ast = Ast::new_with_val(AstType::AssignExpr, self.curr_val.clone());

        // Advance to type declaration
        self.get_next_tok();

        if self.curr_type != TokenType::Colon {
            panic!("tank: Parse error - Expected ':'");
        }

        // Advance to type name
        self.get_next_tok();

        if self.curr_type != TokenType::Ident {
            panic!("tank: Parse error - Expected an identifier");
        }

        // Check if type is valid (ie. exists)
        match self.lexer.reserved.words.get(&self.curr_val) {
            Some(_) => (),
            None => panic!("tank: Invalid type provided for variable declaration")
        }

        assign_ast.var_type = Some(self.curr_val.clone());

        // advance to equals symbol
        self.get_next_tok();

        if self.curr_type != TokenType::Equals {
            panic!("tank: Invalid assignment expression");
        }

        // advance to assignment value
        self.get_next_tok();

        // TODO: normal expression parsing in assignment statements
        match self.curr_type {
            TokenType::Ident => assign_ast.children.push(self.ident()),
            TokenType::Number => assign_ast.children.push(self.number()),
            _ => panic!("tank: invalid variable assignment value provided")
        }

        Box::new(assign_ast)
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
