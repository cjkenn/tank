extern crate serde;
extern crate serde_json;

use std::fs::File;
use std::error::Error;
use std::io::Read;
use std::collections::BTreeMap;

use syntax::parser::Parser;
use syntax::symbol_table::SymbolTable;
use generate::gen::Gen;

pub struct Compiler {
    parser: Parser,
    filename: String
}

impl Compiler {
    pub fn new(m_file: &mut File, filename: &String) -> Compiler {
        let mut file_contents = String::new();

        match m_file.read_to_string(&mut file_contents) {
            Err(error) => panic!("Failed to read {}: {}",
                                 &filename,
                                 Error::description(&error)),
            Ok(_) => ()
        }

        let sym_tab = SymbolTable::new();
        let parser = Parser::new(file_contents, sym_tab);

        Compiler {
            parser: parser,
            filename: filename.to_owned()
        }
    }

    /// Create a new compiler using a JSON "config" file. This file is
    /// expected to contain other variables or information not declared
    /// within the template expected to be compiled. The scoping of these
    /// config vars will be global for the current file.
    pub fn from_config_file(m_file: &mut File,
                            filename: &String,
                            config_file: &mut File) -> Compiler {
        let mut config_file_contents = String::new();
        match config_file.read_to_string(&mut config_file_contents) {
            Err(error) => panic!("Failed to read config file: {}",
                                 Error::description(&error)),
            Ok(_) => ()
        }

        let input_map: BTreeMap<String, String> = serde_json::from_str(&config_file_contents)
            .unwrap();
        let sym_tab = SymbolTable::from_existing_map(&input_map);
        let mut file_contents = String::new();

        match m_file.read_to_string(&mut file_contents) {
            Err(error) => panic!("Failed to read {}: {}",
                                 &filename,
                                 Error::description(&error)),
            Ok(_) => ()
        }

        let parser = Parser::new(file_contents, sym_tab);

        Compiler {
            parser: parser,
            filename: filename.to_owned()
        }
    }

    /// Given a file and a parser created by the new functions,
    /// this function compiles a .tank file and writes the output
    /// to the corresponding .html file.
    pub fn compile(&mut self) -> &Compiler {
        println!("tank: Compiling '{}'...", &self.filename);
        self.parser.parse();

        if self.parser.messages.has_messages() {
            self.parser.messages.print_messages();

            // We will panic here if there are errors. There is no need
            // to continue trying to generate an output if we know we haven't
            // been able to parse...
            if self.parser.messages.is_err() {
                panic!("tank: Could not compile {}", &self.filename)
            }
        }

        let ast = &self.parser.root;
        let sym = self.parser.symbol_table.clone();

        let mut gen = Gen::new(&self.filename, sym);
        gen.output(ast.clone());

        self
    }
}
