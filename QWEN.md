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

1. `==`, `!=` (equality/inequality)
2. `&` (bitwise AND)
3. `^` (bitwise XOR)
4. `|` (bitwise OR)
5. `&&` (logical AND)
6. `||` (logical OR)

Use parentheses to override default precedence.
