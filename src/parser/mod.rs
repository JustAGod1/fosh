pub mod ast;
mod cmd;

pub use cmd::*;


#[cfg(test)]
mod tests {
    use crate::parser::ast::{ASTKind, ASTNode};
    use crate::parser::CmdParser;

    #[test]
    fn correct_function_call() {
        let input = "&k";

        let mut error = false;
        let node = CmdParser::new().parse(&mut error, input).unwrap();
        assert!(!error);

        assert_eq!(node.value.as_ref().unwrap().kind().clone(), ASTKind::FunctionCall);
        assert_eq!(node.children.get(0).unwrap().value.as_ref().unwrap().kind().clone(), ASTKind::Ampersand);
        assert_eq!(node.children.get(1).unwrap().value.as_ref().unwrap().kind().clone(), ASTKind::FunctionName);
    }

    #[test]
    fn correct_edge_walk_correct_input() {
        correct_edge_walk("&f")
    }

    #[test]
    fn correct_edge_walk_incorrect_input() {
        correct_edge_walk("f&f")
    }

    fn correct_edge_walk(input: &str) {
        let mut error = false;
        let node = CmdParser::new().parse(&mut error, input).unwrap();
        assert!(!error);

        let mut s = String::new();

        node.walk(&mut |n| {
            if n.is_leaf() { s.push_str(n.span.slice(input)) }
        });

        assert_eq!(input, s)
    }

    #[test]
    fn incorrect_with_correct_tail() {
        let input = "f&f";

        let mut error = false;
        let node = CmdParser::new().parse(&mut error, input).unwrap();
        assert!(!error);



    }


}