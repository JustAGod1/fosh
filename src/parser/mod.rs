pub mod ast;
mod tokenizer;
mod cmd;

#[cfg(not(test))]
use cmd::*;
#[cfg(test)]
pub use cmd::*;

use crate::parser::ast::{ASTKind, ASTNode};
use crate::parser::tokenizer::Tokenizer;

trait ParserAdapter {
    fn parse(&self, cmd: &str) -> Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>>;

    #[cfg(test)]
    fn parse_with_tokens(&self, tokens: Vec<Result<(usize, ASTKind, usize), (usize, usize)>>) -> Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>>;
}

macro_rules! impl_adapter {
    ($t:ty) => {
        impl ParserAdapter for $t {
            fn parse(&self, cmd: &str) -> Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>> {
                let tokens = tokenizer::Tokenizer::new(cmd);
                return self.parse(tokens);
            }

            #[cfg(test)]
            fn parse_with_tokens(&self, tokens: Vec<Result<(usize, ASTKind, usize), (usize, usize)>>) -> Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>> {
                return self.parse(tokens.into_iter());
            }
        }
    };
}

impl_adapter!(DelimitedParser);
impl_adapter!(PropertyCallNodeParser);
impl_adapter!(ValueParser);


pub fn parse(data: &str) -> Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>> {
    let parser = DelimitedParser::new();
    return parser.parse(Tokenizer::new(data));
}


#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fmt::{Debug, format};
    use crate::parser::tokenizer::tests::*;
    use logos::{Logos, Source};
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    use rand::seq::IteratorRandom;
    use crate::annotator::parse_tree::ParseTree;
    use crate::parser::ast::{ASTNode, Span};
    use crate::parser::ast::*;
    use crate::parser::cmd::DelimitedParser;
    use crate::parser::{ParserAdapter, PropertyCallNodeParser};
    use crate::parser::tokenizer::tests::tokenize;
    use crate::parser::tokenizer::Tokenizer;

    static TERMINALS : [ASTKind;15] = [
        ASTKind::Ampersand,
        ASTKind::Pipe,
        ASTKind::SemiColon,
        ASTKind::Dollar,
        ASTKind::OpenParen,
        ASTKind::OpenParen,
        ASTKind::CloseParen,
        ASTKind::OpenBrace,
        ASTKind::CloseBrace,
        ASTKind::NumberLiteral,
        ASTKind::Dot,
        ASTKind::Comma,
        ASTKind::Literal,
        ASTKind::Identifier,
        ASTKind::DoubleQuote
    ];

    pub fn build_pt_def(data: &str) -> ParseTree {
        ParseTree::new(data, build_ast(DelimitedParser::new(), data))
    }
    fn build_pt<T: ParserAdapter>(adapter: T, data: &str) -> ParseTree {
        ParseTree::new(data, build_ast(adapter, data))
    }
    fn build_ast<T: ParserAdapter>(adapter: T, data: &str) -> ASTNode {
        adapter.parse(data).unwrap()
    }

    fn assert_parsed_with_errors(data: &str) {
        let ast = DelimitedParser::new().parse(Tokenizer::new(data));

        validate_ast_error(data, ast);
    }

    fn validate_ast_error(data: &str, ast: Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>>) {
        assert!(ast.is_ok(), "Parsing failed: {}\n Error: {:?}", data, ast);

        println!("{:?}", ast.as_ref()
            .unwrap());

        let pt = ParseTree::new(data, ast.unwrap());
        let error = pt.root().find_child_with_kind_rec(ASTKind::Error);
        assert!(error.is_some(), "No error found in parse tree");

    }
    fn validate_ast(data: &str, ast: Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>>) {
        assert!(ast.is_ok(), "Parsing failed: {}\n Error: {:?}", data, ast);

        let ast_str = format!("{:?}", ast.as_ref().unwrap());

        let pt = ParseTree::new(data, ast.unwrap());
        let error = pt.root().find_child_with_kind_rec(ASTKind::Error);
        assert!(error.is_none(), "Parsing failed:\n{}\nError: {:?}\nTree: {}", data, error.unwrap().origin, ast_str);
    }

    fn assert_parsed(data: &str) {
        let ast = DelimitedParser::new().parse(Tokenizer::new(data));

        validate_ast(data, ast);
    }

    fn assert_parsed_special<T : ParserAdapter>(adapter: T, tokens: Vec<ASTKind>)
    {
        let data = format!("{:?}", tokens);
        let tokens = tokens.into_iter().map(|x| Ok((0usize, x, 0usize)))
            .collect::<Vec<Result<(usize, ASTKind, usize), (usize, usize)>>>();
        let ast = adapter.parse_with_tokens(tokens);
        validate_ast(&data, ast);
    }

    fn assert_parsed_with_errors_special<T: ParserAdapter>(adapter: T, tokens: Vec<ASTKind>) {
        let data = format!("{:?}", tokens);
        let tokens = tokens.into_iter().map(|x| Ok((0usize, x, 0usize)))
            .collect::<Vec<Result<(usize, ASTKind, usize), (usize, usize)>>>();
        let ast = adapter.parse_with_tokens(tokens);
        validate_ast_error(&data, ast);
    }

    #[test]
    fn fuzzing_no_error_tokens_always_parsed_expected() {
        fuzzing(&TERMINALS);
    }

    #[test]
    fn fuzzing_with_error_tokens_always_parsed_expected() {
        let mut terminals = TERMINALS.to_vec();
        terminals.push(ASTKind::Error);
        fuzzing(&terminals);
    }

    fn fuzzing(terminals: &[ASTKind]) {
        let mut rng = SmallRng::from_seed([77u8; 32]);

        let iterations = rng.gen_range(50u32..100);

        for _ in 0..iterations {
            let mut tokens: Vec<Result<(usize, ASTKind, usize), (usize, usize)>> = Vec::new();
            for i in 0..rng.gen_range(1usize..10) {
                let token = terminals.iter().choose(&mut rng).unwrap();
                tokens.push(Ok((i, *token, i+1)));
            }

            let error_msg = &format!("Parsing failed.\nTokens: {:?}", tokens);
            DelimitedParser::new()
                .parse(tokens.into_iter())
                .expect(error_msg);

        }



    }

    #[test]
    fn test_leftmost_recursion() {
        let pt = build_pt(DelimitedParser::new(), "$kek.lol.arbidol");

        let node = pt.root().find_child_with_kind_rec(ASTKind::PropertyCall).unwrap();
        let call: &PropertyCall = node.value();

        assert_eq!(call.left_hand(node).unwrap().text(), "kek.lol");
        assert_eq!(call.get_property_name(node).unwrap(), "arbidol");

    }


    #[test]
    fn test_parse_empty() {
        assert_parsed_with_errors("");
    }

    #[test]
    fn test_parse_simple_property() {
        assert_parsed("$foo");
    }

    #[test]
    fn test_property_call() {
        assert_parsed_special(
            PropertyCallNodeParser::new(),
            tokenize_function_level("kek()")
        );
        assert_parsed_special(
            PropertyCallNodeParser::new(),
            tokenize_function_level("kek")
        );
        assert_parsed_with_errors_special(
            PropertyCallNodeParser::new(),
            tokenize_function_level("kek(")
        );
    }

    #[test]
    fn test_value() {
        assert_parsed_special(
            ValueParser::new(),
            tokenize_function_level(r#" "fd" "#)
        );
        assert_parsed_special(
            ValueParser::new(),
            tokenize_function_level(r#" 439 "#)
        );
        assert_parsed_special(
            ValueParser::new(),
            tokenize_function_level(r#" {lol} "#)
        );
        assert_parsed_special(
            ValueParser::new(),
            tokenize_function_level(r#" {lol}.kek() "#)
        );
        assert_parsed_special(
            ValueParser::new(),
            tokenize_function_level(r#" kek() "#)
        );
    }

    #[test]
    fn test_parse_single_value() {
        assert_parsed("$5");
        assert_parsed(r#"$"ffdf""#);
        assert_parsed(r#"5.0"#);
    }

    #[test]
    fn test_parse_property_invocation_no_arg() {
        assert_parsed("$foo");
        assert_parsed("$foo()");
        assert_parsed("$foo.kek");
        assert_parsed("$foo.kek.lol");
    }

    #[test]
    fn test_parse_property_invocation_one_arg() {
        assert_parsed("$foo(5)");
        assert_parsed(r#"$foo("kek")"#);
        assert_parsed(r#"$foo(5.0)"#);
    }

    #[test]
    fn test_parse_property_invocation_some_args() {
        // Homogeneous
        assert_parsed("$foo(5 5)");
        assert_parsed(r#"$foo("kek" "lol" "arbidol")"#);
        assert_parsed(r#"$foo(5.0 88.9 84.0)"#);

        // Heterogeneous
        assert_parsed(r#"$foo(5 "fdf" 8.9)"#);
    }

    #[test]
    fn test_parse_chain_function() {
        assert_parsed(r#"$foo.kek()"#);
        assert_parsed(r#"$foo().kek()"#);
        assert_parsed(r#"$foo(543).kek()"#);
        assert_parsed(r#"$foo().kek("fd")"#);
    }

    #[test]
    fn test_parse_delimited() {
        assert_parsed(r#"$foo ; echo"#);
        assert_parsed(r#"$foo.lol ; echo"#);
        assert_parsed(r#"$foo.lol("fdfda") ; echo ; $fdfd"#);
    }

    #[test]
    fn test_parse_piped() {
        assert_parsed(r#"$lol"#);
        assert_parsed(r#"$lol | echo"#);
        assert_parsed(r#"$lol | echo | kek"#);
    }

    #[test]
    fn test_parse_sequenced() {
        assert_parsed(r#"$lol"#);
        assert_parsed(r#"$lol & echo"#);
        assert_parsed(r#"$lol & echo & kek"#);
    }

    #[test]
    fn test_parse_several_delimiters() {
        assert_parsed(r#"$lol"#);
        assert_parsed(r#"$lol & echo"#);
        assert_parsed(r#"$lol & echo | kek"#);
        assert_parsed(r#"$lol & echo | kek ; cheburek"#);
    }

    #[test]
    fn test_parse_braced_commands() {
        assert_parsed(r#"${lol}"#);
        assert_parsed_with_errors(r#"${}"#);

        assert_parsed(r#"${lol}.kek()"#);
        assert_parsed(r#"${lol}.kek() ; lol"#);
    }

    #[test]
    fn test_assignation() {
        assert_parsed(r#"$foo = 5"#);
        assert_parsed(r#"$foo = "kek""#);
        assert_parsed(r#"$foo = 5.0"#);
        assert_parsed(r#"$foo = {lol}"#);
        assert_parsed(r#"$foo = {lol}.kek()"#);
        assert_parsed(r#"$foo = {lol}.kek("fd")"#);
        assert_parsed(r#"$foo = {lol}.kek("fd" "lol")"#);
    }

    #[test]
    fn parse_string_literal() {
        let pt = build_pt_def(r#"$kek("kek")"#);

        pt.root();

        let node = pt.root().find_child_with_kind_rec(ASTKind::StringLiteral).unwrap();
        assert_eq!(node.data, r#""kek""#)
    }

    #[test]
    fn handmade() {
        assert_parsed(r#"$foo({kek} lol.lmao 59 {kek | $lol()})"#)
    }

}
