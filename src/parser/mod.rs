pub mod ast;
mod tokenizer;
mod cmd;

use crate::parser::ast::{ASTKind, ASTNode};
use cmd::*;
use crate::parser::tokenizer::Tokenizer;

trait ParserAdapter {
    fn parse(&self, cmd: &str) -> Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>>;
}

impl ParserAdapter for DelimitedParser {
    fn parse(&self, cmd: &str) -> Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>> {
        let tokens = tokenizer::Tokenizer::new(cmd);
        return self.parse(tokens);
    }
}

pub fn parse(data: &str) -> Result<ASTNode, lalrpop_util::ParseError<usize, ASTKind, (usize, usize)>> {
    let parser = DelimitedParser::new();
    return parser.parse(Tokenizer::new(data));
}


#[cfg(test)]
pub mod tests {
    use std::fmt::format;
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    use rand::seq::IteratorRandom;
    use crate::annotator::parse_tree::ParseTree;
    use crate::parser::ast::{ASTNode, Span};
    use crate::parser::ast::*;
    use crate::parser::cmd::DelimitedParser;
    use crate::parser::ParserAdapter;
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
        ASTKind::StringLiteral,
        ASTKind::NumberLiteral,
        ASTKind::Dot,
        ASTKind::Comma,
        ASTKind::Literal,
        ASTKind::Identifier,
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

    fn assert_parsed(data: &str) {
        let ast = DelimitedParser::new().parse(Tokenizer::new(data));

        assert!(ast.is_ok(), "Parsing failed: {}\n Error: {:?}", data, ast);

        println!("{:?}", ast.as_ref()
            .unwrap());

        let pt = ParseTree::new(data, ast.unwrap());
        let error = pt.root().find_child_with_kind_rec(ASTKind::Error);
        assert!(error.is_none(), "Parsing failed: {}\n Error: {:?}", data, error.unwrap().origin);
    }

    #[test]
    fn fuzzing_parsing_always_parsed_expected() {
        let mut rng = SmallRng::from_seed([77u8; 32]);

        let iterations = rng.gen_range(50u32..100);

        for _ in 0..iterations {
            let mut tokens: Vec<Result<(usize, ASTKind, usize), (usize, usize)>> = Vec::new();
            for i in 0..rng.gen_range(1usize..10) {
                let token = TERMINALS.iter().choose(&mut rng).unwrap();
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

        let node = pt.root().find_child_with_kind_rec(ASTKind::CallChain).unwrap();
        let chain : &CallChain = node.value();

        assert_eq!(chain.get_left_hand(node).unwrap().text(), "kek.lol");
        assert_eq!(chain.get_right_hand(node).unwrap().text(), "arbidol");

    }


    #[test]
    fn test_parse_empty() {
        assert_parsed("");
    }

    #[test]
    fn test_parse_simple_property() {
        assert_parsed("$foo");
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

}
