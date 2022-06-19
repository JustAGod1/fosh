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
mod tests {
    use crate::parser::ast::{ASTNode, Span};
    use crate::parser::ast::*;
    use crate::parser::cmd::DelimitedParser;
    use crate::parser::ParserAdapter;
    use crate::parser::tokenizer::Tokenizer;

    fn build_ast<T: ParserAdapter>(adapter: T, data: &str) -> ASTNode {
        adapter.parse(data).unwrap()
    }

    fn assert_parsed(data: &str) {
        let ast = DelimitedParser::new().parse(Tokenizer::new(data));

        assert!(ast.is_ok(), "Parsing failed: {}\n Error: {:?}", data, ast.err().unwrap());
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
    fn test_parse_several_modes() {
        assert_parsed(r#"$foo ; echo"#);
        assert_parsed(r#"$foo.lol ; echo"#);
        assert_parsed(r#"$foo.lol("fdfda") ; echo ; $fdfd"#);
    }

    #[test]
    fn simple() {
        let ast = build_ast(DelimitedParser::new(), "echo hello");
        let expected = ASTNode {
            span: Span::new(0, 10),
            value: CommandLine::new().boxed(),
            children: vec![
                ASTNode {
                    span: Span::new(0,10),
                    value: Command::new().boxed(),
                    children: vec![
                        ASTNode {
                            span: Span::new(0, 4),
                            value: CommandName::new().boxed(),
                            children: vec![]
                        },
                        ASTNode {
                            span: Span::new(5, 10),
                            value: CommandArguments::new().boxed(),
                            children: vec![
                                ASTNode {
                                    span: Span::new(5, 10),
                                    value: Literal::new().boxed(),
                                    children: vec![]
                                }
                            ]
                        }
                    ]
                }
            ]
        };
        assert_eq!(ast, expected);
    }
}
