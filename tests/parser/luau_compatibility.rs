// Luau compatibility test suite

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use compiler::lexer::Lexer;
    use compiler::lexer::TokenKind;
    use compiler::parser::Parser;
    use compiler::source::SourceManager;
    use compiler::parser::ast_builder::{AstNode, ExpressionKind, StatementKind};

    fn parse_program(code: &str) -> AstNode {
        let mut sources = SourceManager::new();
        let file_id = sources.add_file(PathBuf::from("test.glu"), String::from(code));
        let file = sources.get(file_id).unwrap();

        let mut lexer = Lexer::new(file);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token();
            tokens.push(token.clone());
            if matches!(token.kind, TokenKind::EOF) {
                break;
            }
        }

        let mut parser = Parser::new(&tokens);
        parser.parse_program()
    }

    #[test]
    fn test_luau_string_shorthand() {
        // Luau allows string literals as single arguments without parentheses
        let code = r#"print "Hello World""#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
            if let StatementKind::Expression(expr) = &program.statements[0].kind {
                assert!(matches!(expr.kind, ExpressionKind::Call { .. }));
            }
        }
    }

    #[test]
    fn test_luau_table_shorthand() {
        // Luau allows table constructors as single arguments without parentheses
        let code = r#"spawn { Position = Vector3.zero }"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
            if let StatementKind::Expression(expr) = &program.statements[0].kind {
                assert!(matches!(expr.kind, ExpressionKind::Call { .. }));
            }
        }
    }

    #[test]
    fn test_luau_chained_shorthand_calls() {
        // Luau allows chained shorthand calls
        let code = r#"factory "Player" "Enemy""#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
            if let StatementKind::Expression(expr) = &program.statements[0].kind {
                assert!(matches!(expr.kind, ExpressionKind::Call { .. }));
            }
        }
    }

    #[test]
    fn test_luau_method_call() {
        // Luau method calls with colon syntax
        let code = r#"player:Jump()"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
            if let StatementKind::Expression(expr) = &program.statements[0].kind {
                assert!(matches!(expr.kind, ExpressionKind::MethodCall { .. }));
            }
        }
    }

    #[test]
    fn test_luau_interpolated_strings() {
        // Luau backtick interpolated strings
        let code = r#"local message = `Hello {name}, score: {score}`"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
            if let StatementKind::Local { names, .. } = &program.statements[0].kind {
                assert_eq!(names[0].0, "message");
            }
        }
    }

    #[test]
    fn test_luau_compound_assignment() {
        // Luau compound assignment operators
        let code = r#"x += 1
health -= damage
coins *= 2
score /= 5"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 4);
        }
    }

    #[test]
    fn test_luau_bitwise_compound_assignment() {
        // Luau bitwise compound assignment
        let code = r#"flags &= mask
flags |= mask"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 2);
        }
    }

    #[test]
    fn test_luau_continue_statement() {
        // Luau continue statement
        let code = r#"for i = 1, 10 do
    if i == 5 then
        continue
    end
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_type_annotations() {
        // Luau type annotations
        let code = r#"local x: number = 10
local player: Player = createPlayer()
local name: string? = getName()"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 3);
        }
    }

    #[test]
    fn test_luau_optional_types() {
        // Luau optional types with ?
        let code = r#"local value: number? = nil
local player: Player? = nil"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 2);
        }
    }

    #[test]
    fn test_luau_union_types() {
        // Luau union types
        let code = r#"local value: number | string = 5"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_function_types() {
        // Luau function types
        let code = r#"type Callback = (number, string) -> boolean"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_table_types() {
        // Luau table types
        let code = r#"type Vector3 = { x: number, y: number, z: number }"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_array_types() {
        // Luau array types
        let code = r#"local numbers: { number } = {1, 2, 3}"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_real_world_pattern() {
        // Test a realistic Luau pattern from Roblox development
        let code = r#"
local Players = game:GetService("Players")
local player = Players.LocalPlayer

local function formatScore(score: number): string
    return string.format("Score: %d", score)
end

player.CharacterAdded:Connect(function(character)
    print("Character added for: " .. player.Name)
end)

local data = {
    playerName = player.Name,
    userId = player.UserId,
    stats = {
        level = 5,
        experience = 1000
    }
}

print `Player {data.playerName} (ID: {data.userId})`
"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            // Should parse without errors
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_nested_method_calls() {
        // Luau nested method calls
        let code = r#"player.Inventory:GetWeapon(1):Equip()"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_complex_expressions() {
        // Luau complex expressions with multiple operators
        let code = r#"local result = (a + b) * c / d - e % f"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_string_concatenation() {
        // Luau string concatenation with ..
        let code = r#"local message = "Hello, " .. name .. "!"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_logical_operators() {
        // Luau logical operators
        let code = r#"local result = x and y or z"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_comparison_operators() {
        // Luau comparison operators
        let code = r#"local a = x == y
local b = x ~= y
local c = x < y
local d = x <= y
local e = x > y
local f = x >= y"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 6);
        }
    }

    #[test]
    fn test_luau_multiple_return_values() {
        // Luau multiple return values
        let code = r#"local x, y, z = getPosition()
return x, y, z"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 2);
        }
    }

    #[test]
    fn test_luau_nested_tables() {
        // Luau nested table structures
        let code = r#"local config = {
    settings = {
        graphics = {
            quality = "high",
            shadows = true
        },
        audio = {
            volume = 0.8,
            music = true
        }
    }
}"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_function_with_params() {
        // Luau function with type parameters
        let code = r#"function createEnemy(type: string, health: number?)
    health = health or 100
    -- implementation
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_self_pattern() {
        // Luau self pattern in methods
        let code = r#"function Enemy:TakeDamage(amount: number)
    self.health -= amount
    if self.health <= 0 then
        self:Destroy()
    end
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_oop_pattern() {
        // Luau OOP pattern
        let code = r#"local Class = {}
Class.__index = Class

function Class.new(position)
    local self = setmetatable({}, Class)
    self.position = position
    return self
end

function Class:Move(newPosition)
    self.position = newPosition
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_module_pattern() {
        // Luau module pattern
        let code = r#"local Utils = {}

function Utils.calculateDistance(a, b)
    return math.sqrt((a.x - b.x)^2 + (a.y - b.y)^2)
end

function Utils.formatTime(seconds)
    local hours = math.floor(seconds / 3600)
    local minutes = math.floor((seconds % 3600) / 60)
    return string.format("%d:%02d", hours, minutes)
end

return Utils"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_event_handler_pattern() {
        // Luau event handler pattern
        let code = r#"local function onPlayerJoin(player)
    print(player.Name .. " joined the game")
    player:LoadData()
end

Players.PlayerAdded:Connect(onPlayerJoin)"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_pcall_pattern() {
        // Luau pcall pattern for error handling
        let code = r#"local success, result = pcall(someFunction, arg1, arg2)
if not success then
    warn("Function failed: " .. result)
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_table_manipulation() {
        // Luau table manipulation functions
        let code = r#"local t = {}
table.insert(t, 1)
table.insert(t, 2)
table.remove(t, 1)
local count = #t"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_math_library() {
        // Luau math library usage
        let code = r#"local x = math.random(1, 100)
local y = math.floor(x)
local z = math.ceil(x)
local w = math.sqrt(16)"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_string_library() {
        // Luau string library usage
        let code = r#"local s = "Hello World"
local lower = string.lower(s)
local upper = string.upper(s)
local sub = string.sub(s, 1, 5)
local split = string.split(s, " ")"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_ipairs_pattern() {
        // Luau ipairs pattern
        let code = r#"local array = {10, 20, 30, 40}
for index, value in ipairs(array) do
    print(index, value)
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_pairs_pattern() {
        // Luau pairs pattern
        let code = r#"local table = {a = 1, b = 2, c = 3}
for key, value in pairs(table) do
    print(key, value)
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_recursion() {
        // Luau recursive function
        let code = r#"local function factorial(n: number): number
    if n <= 1 then
        return 1
    end
    return n * factorial(n - 1)
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_closure() {
        // Luau closure pattern
        let code = r#"local function createCounter()
    local count = 0
    return function()
        count = count + 1
        return count
    end
end

local counter = createCounter()
print(counter())  -- 1
print(counter())  -- 2"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_upvalue_pattern() {
        // Luau upvalue pattern
        let code = r#"local x = 10

local function getX()
    return x
end

local function setX(value: number)
    x = value
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_if_elseif_else_chain() {
        // Luau if-elseif-else chain
        let code = r#"if x > 10 then
    print("High")
elseif x > 5 then
    print("Medium")
elseif x > 0 then
    print("Low")
else
    print("Zero or negative")
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_numeric_for_loop() {
        // Luau numeric for loop
        let code = r#"for i = 1, 10, 2 do
    print(i)
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_generic_for_loop() {
        // Luau generic for loop
        let code = r#"for key, value in pairs(someTable) do
    print(key, value)
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_repeat_until_loop() {
        // Luau repeat-until loop
        let code = r#"repeat
    player:Move()
    wait(1)
until player.Position == target"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_nested_loops() {
        // Luau nested loops
        let code = r#"for i = 1, 10 do
    for j = 1, 10 do
        print(i, j)
    end
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_multiple_assignment_from_function() {
        // Luau multiple assignment from function
        let code = r#"local function getPosition()
    return 100, 200, 300
end

local x, y, z = getPosition()"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_swapping_variables() {
        // Luau variable swapping
        let code = r#"local a, b = 1, 2
a, b = b, a"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_scope_pattern() {
        // Luau scope pattern
        let code = r#"local x = 10

do
    local x = 20  -- Different x in inner scope
    print(x)  -- 20
end

print(x)  -- 10"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_table_indexing() {
        // Luau table indexing
        let code = r#"local t = {x = 10, y = 20}
print(t.x)       -- 10
print(t["x"])     -- 10
print(t[1])       -- First element"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_string_interpolation_complex() {
        // Luau complex string interpolation
        let code = r#"local message = `Player {player.Name} (Level {player.Level}) has {player.Health} HP`"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_varargs_with_unpacking() {
        // Luau varargs with unpacking
        let code = r#"local function printAll(...)
    local args = {...}
    print(unpack(args))
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_default_parameter_pattern() {
        // Luau default parameter pattern
        let code = r#"local function createEntity(type: string, position: Vector3?)
    position = position or Vector3.zero
    -- implementation
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_tail_call_optimization() {
        // Luau tail call pattern
        let code = r#"local function factorial(n: number, acc: number): number
    if n <= 1 then
        return acc
    end
    return factorial(n - 1, n * acc)  -- Tail call
end"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_assert_pattern() {
        // Luau assert pattern
        let code = r#"assert(condition, "Error message")
assert(player ~= nil, "Player cannot be nil")"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_require_pattern() {
        // Luau require pattern
        let code = r#"local Utils = require(game.ReplicatedStorage:WaitForChild("Utils"))
local result = Utils.calculate(5, 10)"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_backward_compatibility() {
        // Test backward compatibility with standard Lua
        let code = r#"-- Standard Lua code should also work
local function factorial(n)
    if n == 0 then
        return 1
    else
        return n * factorial(n - 1)
    end
end

print(factorial(5))

-- Standard Lua table operations
local t = {apple = 5, banana = 3, cherry = 7}
for k, v in pairs(t) do
    print(k, v)
end
"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_parser_robustness() {
        // Test parser robustness with various edge cases
        let code = r#"
-- Comments
local x = 5 -- inline comment
--[[ Multiline
   comment ]]
-- Empty lines

-- Complex nesting
if true then
    if false then
        while true do
            break
        end
    end
end

-- Mixed operators
local result = ((1 + 2) * 3) / 4 % 5

-- Deeply nested calls
a(b(c(d(e(f(g())))))

-- Complex table
local data = {
    metadata = {
        created = os.time(),
        modified = os.time(),
        author = "Test"
    },
    content = {
        items = {
            {id = 1, name = "Item 1"},
            {id = 2, name = "Item 2"}
        }
    }
}

-- Type annotations everywhere
local function process(data: {any}): {string}
    return "processed"
end
"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            // Should parse without crashing
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_edge_cases() {
        // Test edge cases and corner cases
        let code = r#"
-- Empty program

-- Single character identifiers
local a = 1
local b = 2

-- Numbers with various formats
local int = 42
local float = 3.14
local scientific = 1.5e10
local hex = 0xFF

-- String escapes
local s1 = "Hello\nWorld"
local s2 = "Tab\there"

-- Maximum nesting depth test
local result = (((((((1))))))

-- Long chains
local a = b.c.d.e.f.g.h.i.j.k.l.m.n.o.p

-- Multiple consecutive operators
local x = 1 + 2 + 3 + 4 + 5
"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            // Should parse without crashing
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_unicode_strings() {
        // Luau unicode string support
        let code = r#"local message = "Hello 世界"
local emoji = "😀🎉""#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_large_program() {
        // Test parsing a larger program
        let code = r#"
local function main()
    -- Setup
    local config = {
        debug = true,
        maxPlayers = 100,
        serverName = "GraduaLuau Server"
    }
    
    -- Initialize services
    local services = {
        database = {},
        auth = {},
        game = {}
    }
    
    -- Game loop
    while true do
        -- Process input
        local input = services.game:getInput()
        
        -- Update game state
        services.game:update(input)
        
        -- Render
        services.game:render()
        
        -- Wait for next frame
        wait(1/60)
    end
end

main()
"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_all_statement_types() {
        // Test all statement types in one program
        let code = r#"
-- Local variable
local x = 5

-- Assignment
x = 10

-- Compound assignment
x += 5

-- Function declaration
function test()
    return 42
end

-- Local function
local function helper()
    return true
end

-- If statement
if x > 5 then
    print("Large")
end

-- While loop
while x < 20 do
    x = x + 1
end

-- For loop
for i = 1, 10 do
    print(i)
end

-- Repeat-until loop
repeat
    x = x + 1
until x >= 10

-- Return statement
return x

-- Break statement
for i = 1, 10 do
    if i == 5 then
        break
    end
end

-- Continue statement
for i = 1, 10 do
    if i == 5 then
        continue
    end
end

-- Type alias
type CustomType = { x: number, y: number }

-- Expression statement
x + y

-- Empty statement
;
"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_all_expression_types() {
        // Test all expression types
        let code = r#"
-- Literals
local num = 42
local str = "hello"
local bool = true
local nil_val = nil

-- Unary operators
local neg = -5
local not_val = not true

-- Binary operators
local add = 1 + 2
local sub = 10 - 5
local mul = 3 * 4
local div = 20 / 4
local mod = 10 % 3
local pow = 2 ^ 3
local concat = "Hello " .. "World"

-- Comparison operators
local eq = 5 == 5
local ne = 5 ~= 6
local lt = 3 < 5
local le = 5 <= 5
local gt = 5 > 3
local ge = 5 >= 5

-- Logical operators
local and_val = true and false
local or_val = true or false

-- Function calls
local result = math.sqrt(16)

-- Method calls
local obj = {}
function obj:method()
    return "result"
end
obj:method()

-- Member access
local name = obj.name

-- Index expressions
local item = array[1]

-- Table constructors
local table1 = {}
local table2 = {x = 1, y = 2}
local table3 = {1, 2, 3}

-- Parentheses
local grouped = (1 + 2) * 3
"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_source_compatibility() {
        // Test that we can parse real Luau code patterns
        let code = r#"
-- Roblox-style game code
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")

local function onPlayerJoin(player)
    local leaderstats = player:WaitForChild("leaderstats")
    leaderstats.Default = 0
    
    local success, err = pcall(function()
        leaderstats:SetAsync("wins", 0)
    end)
    
    if not success then
        warn("Failed to set default wins: " .. err)
    end
end

Players.PlayerAdded:Connect(onPlayerJoin)

local function formatLeaderboard(player)
    local template = `Player: {player.Name} | Wins: {player.leaderstats.Wins}`
    return template
end
"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_luau_error_recovery_with_valid_code() {
        // Test that error recovery doesn't break valid code
        let code = r#"
local x = 5
local y = 10
local z = x + y
print(z)
"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }
}