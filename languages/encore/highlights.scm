(comment) @comment

(function_attribute
  "#" @punctuation.special
  name: (identifier) @attribute)

(decorator_application "@" @punctuation.special)

[
  "fn"
  "async"
  "await"
  "spawn"
  "struct"
  "enum"
  "trait"
  "impl"
  "for"
  "in"
  "let"
  "static"
  "mut"
  "frozen"
  "sending"
  "dyn"
  "ret"
  "while"
  "do"
  "loop"
  "break"
  "continue"
  "if"
  "elif"
  "else"
  "match"
  "import"
  "as"
  "with"
  "macro_rules"
  "extern"
  "unsafe"
  "ehir"
  "not"
] @keyword

(visibility_modifier) @keyword

[
  "+"
  "-"
  "*"
  "**"
  "/"
  "%"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "="
  "+="
  "-="
  "*="
  "**="
  "/="
  "%="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "&&"
  "||"
  "&"
  "|"
  "^"
  "~"
  "<<"
  ">>"
  "?"
  "!"
  "=>"
  "->"
  "::"
  "."
] @operator

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

[
  ","
  ":"
] @punctuation.delimiter

(string_literal) @string
(formatted_string_literal) @string
(formatted_string_text) @string
(formatted_string_interpolation ["{" "}"] @punctuation.special)
(integer_literal) @number
(float_literal) @number.float
(boolean_literal) @boolean
(numeric_suffix) @type.builtin

(smart_pointer_suffix) @type.builtin
(any_pointer_suffix) @operator

(typed_parameter name: (identifier) @variable.parameter)
(closure_parameter name: (identifier) @variable.parameter)
(receiver_parameter "self" @variable.special)
(match_binding name: (identifier) @variable.parameter)
(for_statement item: (identifier) @variable.parameter)
(with_statement binding: (identifier) @variable.parameter)
(let_statement name: (identifier) @variable)
(global_let_statement name: (identifier) @variable)
(global_static_statement name: (identifier) @constant)
(loop_label name: (identifier) @label)

((assignment_target
   (path (path_segment name: (identifier) @variable)))
 (#match? @variable "^[a-z_][A-Za-z0-9_]*$"))

((cast_expression
   value: (path (path_segment name: (identifier) @variable)))
 (#match? @variable "^[a-z_][A-Za-z0-9_]*$"))

(import_path module: (identifier) @type)
(import_statement alias: (identifier) @type)

((range_expression
   start: (path (path_segment name: (identifier) @variable)))
 (#match? @variable "^[a-z_][A-Za-z0-9_]*$"))

(function_signature name: (identifier) @function)
(call_expression function: (path (path_segment name: (identifier) @function.call)))
(method_call_expression method: (identifier) @function.method.call)
(macro_rules_definition name: (identifier) @function.macro)
(macro_invocation macro: (path (path_segment name: (identifier) @function.macro)))

(field_access_expression field: (identifier) @property)
(c_like_struct_fields (typed_parameter name: (identifier) @property))

(struct_definition signature: (struct_signature name: (identifier) @type))
(enum_definition name: (identifier) @type)
(trait_definition name: (identifier) @type)
(impl_definition trait_name: (identifier) @type)
(impl_definition target: (type name: (identifier) @type))
(struct_signature name: (identifier) @type)
(type name: (identifier) @type)

((type name: (identifier) @type.builtin)
 (#match? @type.builtin "^(Self|bool|char|str|usize|isize|[ui][0-9]+|f[0-9]+)$"))

(enum_definition (struct_signature name: (identifier) @constructor))
(struct_initializer type: (type name: (identifier) @constructor))

((path_segment name: (identifier) @type)
 (#match? @type "^[A-Z][A-Za-z0-9_]*$"))

((path
   (path_segment name: (identifier) @type)
   "::"
   (path_segment name: (identifier) @constructor))
 (#match? @type "^[A-Z][A-Za-z0-9_]*$")
 (#match? @constructor "^[A-Z][A-Za-z0-9_]*$"))
