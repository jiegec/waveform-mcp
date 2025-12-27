You job is to write a MCP server in Rust.
It's functionality is to open a waveform file, read signals in the waveform.
They are exposes as MCP tools.
You can use the wellen library to parse and read waveform files in VCD or FST formats.
Wellen is at https://github.com/ekiwi/wellen. You use it from crates.io.
Add tests.
Add Github Actions.
Git commit as soon as you have any progress.

## Condition Search Operator Precedence

Operators in condition expressions are evaluated in the following order (highest to lowest):

1. `~`, `!` (bitwise NOT, logical NOT)
2. `==`, `!=` (equality/inequality)
3. `&` (bitwise AND)
4. `^` (bitwise XOR)
5. `|` (bitwise OR)
6. `&&` (logical AND)
7. `||` (logical OR)

Use parentheses to override default precedence.
