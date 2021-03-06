use syntax::lexer::Lexer;
use syntax::token::{Token, TokenType};
use syntax::ast::{Ast, AstType};
use syntax::symbol_table::SymbolTable;
use error::error_traits::Diagnostic;
use error::parse_err::ParseDiagnostic;

pub struct Parser {
    /// Lexer struct called repeatedly to get tokens
    lexer: Lexer,
    /// Current token, held here when a symbol is lexed.
    /// The value and the type of the current symbol can be accessed here
    curr_tok: Token,
    /// Our current symbol table containing variable declarations
    pub symbol_table: SymbolTable,
    /// Current ast (initially empty)
    pub root: Ast,
    /// Error and warning message information
    pub diagnostic: ParseDiagnostic
}

impl Parser {
    /// Creates a new Parser for given file contents. The file is
    /// expected to be open already and read into a string to be
    /// parsed here.
    ///
    /// This function also lexes the first available character from
    /// the lexer and puts it into the curr_tok field.
    pub fn new(template: String, symbol_table: SymbolTable) -> Parser {
        let mut m_lexer = Lexer::new(template);
        m_lexer.lex();
        let tok = m_lexer.curr_tok.clone();

        Parser {
            lexer: m_lexer,
            symbol_table: symbol_table,
            curr_tok: tok.unwrap_or(Token::new_from_empty()),
            root: Ast::new(AstType::Template),
            diagnostic: ParseDiagnostic::new()
        }
    }

    /// Initiate recursive parsing process. Ast will take the from of Template -> [Element]
    /// here. Template is the top level ast, and should contain any elements that are not
    /// nested in other elements. The parsing process will continually call the lex() method
    /// from the struct's lexer object until EOF is reached.
    pub fn parse(&mut self) -> &mut Parser {
        if self.curr_tok.tok_type == TokenType::Eof {
            self.diagnostic.new_err("End of input reached, nothing to parse!");
        }

        let el = self.element();
        self.root.children.insert(0, el);

        self
    }

    /// Parse and add an Element ast type to the tree. This method is
    /// recursive in all cases, and will call itself until no input remains.
    /// An element ast in tank can contain an html element, a variable assignment,
    /// an if statement or a for-in statement. In the case that we have no elements
    /// left to parse, we will append an EOF to the ast indicating the end of input.
    fn element(&mut self) -> Box<Ast> {
        let mut el_ast = Ast::new(AstType::Element);
        match self.curr_tok.tok_type {
            TokenType::Ident => {

                match self.curr_tok.val.as_ref() {
                    "if" => {
                        // Consume "if"
                        self.get_next_tok();
                        el_ast = Ast::new(AstType::IfExpr);
                        el_ast.children.push(self.expr());

                        // Consume "{"
                        self.expect(TokenType::LeftBrace);

                        el_ast.children.push(self.element());

                        // Consume "}"
                        self.expect(TokenType::RightBrace);

                        let next = self.element();
                        self.root.children.insert(0, next);
                    },
                    "for" => {
                        // Consume "for"
                        self.get_next_tok();
                        el_ast = Ast::new(AstType::ForExpr);
                        let mut first_ident_ast = self.term();

                        self.expect(TokenType::Colon);

                        first_ident_ast.var_type = Some(self.curr_tok.val.clone());
                        // Add to symbol table
                        // TODO: Will eventually be unnecessary I think
                        self.symbol_table.insert_for_id(&first_ident_ast);

                        el_ast.children.push(first_ident_ast);
                        self.get_next_tok();

                        if self.curr_tok.val != "in" {
                            self.diagnostic.new_err("Expected 'in' at for loop");
                        } else {
                            self.get_next_tok();
                        }

                        el_ast.children.push(self.term());
                        el_ast.children.push(self.element());
                    },
                    "let" => {
                        // Consume "let"
                        self.get_next_tok();
                        let assign_el = self.expr();

                        // Add this variable to the symbol table, and panic
                        // if we already tried to declare it before.
                        self.symbol_table.insert(&assign_el);

                        el_ast.children.push(assign_el);
                        let next = self.element();
                        self.root.children.insert(0, next);
                    },
                    _ => {
                        el_ast.children.push(self.term());

                        if self.curr_tok.tok_type == TokenType::LeftParen {
                            el_ast.children.push(self.attr_list());
                        }

                        // Look ahead and see if we have another element
                        if self.peek() == TokenType::LeftParen {
                            el_ast.children.push(self.element());
                        } else {
                            el_ast.children.push(self.contents());
                        }

                        let next = self.element();
                        self.root.children.insert(0, next);
                    }
                };
            },
            TokenType::LeftBrace => {
                // Consume "{"
                self.get_next_tok();

                el_ast.children.push(self.element());

                // Consume "}"
                self.expect(TokenType::RightBrace);
            },
            TokenType::Ampersand => {
                // Consume "&"
                self.get_next_tok();

                el_ast = Ast::new_from_value(AstType::Include, &self.curr_tok.val);

                // Consume filename
                self.get_next_tok();

                let next = self.element();
                self.root.children.insert(0, next);
            },
            _ => {
                el_ast = Ast::new(AstType::Eof);
            }
        }

        Box::new(el_ast)
    }

    /// Parse an attribute list for an html element. An attribute list can contain any number
    /// of desired html attributes, which do not need to be separated by commas (a space is fine).
    /// This method will consume all required punctuation as well.
    fn attr_list(&mut self) -> Box<Ast> {
        let mut attr_ast = Ast::new(AstType::AttrList);

        self.expect(TokenType::LeftParen);

        while self.curr_tok.tok_type != TokenType::RightParen {
            attr_ast.children.push(self.term());

            self.expect(TokenType::Colon);

            attr_ast.children.push(self.term());

            if self.diagnostic.is_err() {
                break;
            }
        }

        self.expect(TokenType::RightParen);

        self.expect(TokenType::Arrow);

        Box::new(attr_ast)
    }

    /// Parse an intial test inside an expression.
    fn expr(&mut self) -> Box<Ast> {
        let mut test_ast = self.op();
        let curr_ast_type = match self.curr_tok.tok_type {
            TokenType::Gt => AstType::Gt,
            TokenType::Lt => AstType::Lt,
            TokenType::GtEquals => AstType::GtEquals,
            TokenType::LtEquals => AstType::LtEquals,
            TokenType::NotEquals => AstType::NotEquals,
            TokenType::EqualsEquals => AstType::EqualsEquals,
            TokenType::Colon => {
                self.get_next_tok();
                test_ast.var_type = Some(self.curr_tok.val.clone());
                // Consume the type.
                self.get_next_tok();

                AstType::AssignExpr
            },
            TokenType::Equals => {
                self.expect(TokenType::Ident);
                AstType::Empty
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

    /// Parse an operation inside an expression.
    fn op(&mut self) -> Box<Ast> {
        let mut op_ast = self.term();

        while self.curr_tok.tok_type == TokenType::Plus || self.curr_tok.tok_type == TokenType::Minus {
            let op_ast_next = op_ast;

            //TODO: Currently, only supporting plus and minus
            let curr_ast_type = match self.curr_tok.tok_type {
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

    /// Method will parse a term in an expression. This can be a constant identifier
    /// or number, or could also contain another expression.
    fn term(&mut self) -> Box<Ast> {
        let term_ast;
        match self.curr_tok.tok_type {
            TokenType::Ident => {
                // If we find a left paren next, we are declaring an element.
                let m_type = match self.peek() {
                    TokenType::LeftParen => AstType::ElementName,
                    _ => AstType::Ident
                };
                term_ast = Box::new(Ast::new_from_value(m_type, &self.curr_tok.val));
                self.get_next_tok();
            },
            TokenType::Number => {
                term_ast = Box::new(Ast::new_from_value(AstType::Number, &self.curr_tok.val));
                self.get_next_tok();
            },
            TokenType::Eof => {
                term_ast = Box::new(Ast::new(AstType::Eof));
            },
            TokenType::Arrow => {
                let err = format!("Unexpected token {:?} found",
                                  self.curr_tok.val);
                self.diagnostic.new_err(&err);
                term_ast = Box::new(Ast::new(AstType::Eof));
            },
            _ => {
                term_ast = self.expr();
            }
        }

        term_ast
    }

    /// Generates the contents of an element by joining together many identifiers
    /// separated by spaces.  Also will consume references to other files within
    /// the element contents and interpolate variables.
    fn contents(&mut self) -> Box<Ast> {
        if self.curr_tok.tok_type == TokenType::Arrow {
            let err = format!("Unexpected token {:?} found",
                              self.curr_tok.val);
            self.diagnostic.new_err(&err);
        }

        match self.curr_tok.tok_type {
            TokenType::Ident => {
                let mut contents_ast = Ast::new(AstType::Contents);

                while (self.curr_tok.tok_type == TokenType::Ident) ||
                    (self.curr_tok.tok_type == TokenType::Percent) {

                    if self.peek() == TokenType::LeftParen {
                        break;
                    }

                    let child = match self.curr_tok.tok_type {
                        TokenType::Ident => Ast::new_from_value(AstType::Ident, &self.curr_tok.val),
                        TokenType::Percent => {
                            self.get_next_tok();
                            Ast::new_from_value(AstType::VariableValue, &self.curr_tok.val)
                        },
                        _ => Ast::new(AstType::Eof)
                    };

                    contents_ast.children.push(Box::new(child));
                    self.get_next_tok();
                }

                Box::new(contents_ast)
            },
            TokenType::Ampersand => {
                // Consume "&"
                self.get_next_tok();

                let include_ast = Box::new(Ast::new_from_value(AstType::Include, &self.curr_tok.val));

                // Consume the identifier for the filename now
                self.get_next_tok();

                include_ast
            },
            TokenType::Percent => {
                // Consume "%"
                self.get_next_tok();

                let var_ast = Box::new(Ast::new_from_value(AstType::VariableValue, &self.curr_tok.val));

                // Consume identifier
                self.get_next_tok();

                var_ast
            },
            _ => {
                Box::new(Ast::new(AstType::Eof))
            }
        }
    }

    /// Match the current token to an expected one. If the current token does not equal
    /// the expected one, the parser will panic. Otherwise, we will advance to the next
    /// token and update the parser internals.
    fn expect(&mut self, token_type: TokenType) {
        if self.curr_tok.tok_type == token_type {
            self.get_next_tok();
        } else {
            let error_str = format!("Expected {:?}, found {:?}",
                                    token_type,
                                    self.curr_tok.tok_type);
            self.diagnostic.parse_err(&error_str, &self.curr_tok);
        }
    }

    /// Retrieve the next available token for parsing. This token is retrieved from the lexer's
    /// lex() method. If the next token from the lexer is None, then we return a token
    /// indicating EOF. We then update the internal value and type fields of the Parser
    /// struct.
    fn get_next_tok(&mut self) -> &mut Parser {
        self.lexer.lex();
        self.curr_tok  = self.lexer.curr_tok.clone().unwrap_or(Token::new_from_empty());

        self
    }

    /// Check the current token but do not consume it.
    fn peek(&self) -> TokenType {
        self.lexer.peek_tok().tok_type
    }
}
