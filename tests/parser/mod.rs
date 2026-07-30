// Parser tests for function-call sugar (ignored until parser is implemented)

#[cfg(test)]
mod tests {
    // These tests are placeholders describing expected parser behavior.
    // They are ignored so they don't fail CI until a parser exists.

    use std::path::PathBuf;

    #[test]
    #[ignore]
    fn parse_string_argument_shorthand() {
        // Source: print "Hello"
        // Expectation: parse as CallExpression(callee=Identifier("print"), args=[StringLiteral("Hello")])
        unimplemented!();
    }

    #[test]
    #[ignore]
    fn parse_table_argument_shorthand() {
        // Source: spawn { Position = Vector3.zero }
        // Expectation: CallExpression(spawn, [TableConstructor{...}])
        unimplemented!();
    }

    #[test]
    #[ignore]
    fn parse_chained_shorthand_calls() {
        // Source: factory "Player" "Enemy"
        // Expectation: nested call expressions
        unimplemented!();
    }

    #[test]
    #[ignore]
    fn parse_mixed_parenthesis_and_shorthand() {
        // Source: print("Hello") "World"
        // Expectation: CallExpression(CallExpression(print, ["Hello"]), ["World"])
        unimplemented!();
    }
}
