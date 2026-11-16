package main

import (
	"fmt"
	"slices"
	"strings"

	"google.golang.org/protobuf/reflect/protoreflect"
)

// Well-known wrappers that prost maps to `Option<scalar>` rather than a struct.
var wrappers = []protoreflect.FullName{
	"google.protobuf.DoubleValue",
	"google.protobuf.FloatValue",
	"google.protobuf.Int64Value",
	"google.protobuf.UInt64Value",
	"google.protobuf.Int32Value",
	"google.protobuf.UInt32Value",
	"google.protobuf.BoolValue",
	"google.protobuf.StringValue",
	"google.protobuf.BytesValue",
}

func renderPair(message protoreflect.MessageDescriptor, spec protoreflect.Message) (string, error) {
	fields := spec.Descriptor().Fields()
	base := spec.Get(fields.ByName("base")).String()
	quote := spec.Get(fields.ByName("quote")).String()
	if base == "" || quote == "" {
		return "", fmt.Errorf("(bolt.options.v1.pair) needs both base and quote")
	}

	b := &body{bindings: map[string]string{}}
	baseTy, baseExpr, err := b.resolve(message, base)
	if err != nil {
		return "", fmt.Errorf("base path `%s`: %w", base, err)
	}
	quoteTy, quoteExpr, err := b.resolve(message, quote)
	if err != nil {
		return "", fmt.Errorf("quote path `%s`: %w", quote, err)
	}

	ty := fmt.Sprintf("(%s, %s)", baseTy, quoteTy)
	expr := fmt.Sprintf("(%s, %s)", baseExpr, quoteExpr)
	if b.fallible {
		ty = fmt.Sprintf("::core::option::Option<%s>", ty)
		expr = fmt.Sprintf("Some(%s)", expr)
	}

	var code strings.Builder
	fmt.Fprintf(&code, "\nimpl %s {\n", rustPath(message))
	fmt.Fprintf(&code, "    /// (base, quote) from `(bolt.options.v1.pair)`: `%s`, `%s`.\n", base, quote)
	if b.fallible {
		code.WriteString("    /// `None` when a message field or `optional` field on either path is unset.\n")
	}
	fmt.Fprintf(&code, "    pub fn pair(&self) -> %s {\n", ty)
	for _, line := range b.lets {
		fmt.Fprintf(&code, "        %s\n", line)
	}
	fmt.Fprintf(&code, "        %s\n", expr)
	code.WriteString("    }\n}\n")
	return code.String(), nil
}

// body accumulates the `let` bindings shared by both paths of a pair, so a message
// field used by both (`instrument.base`, `instrument.quote`) is unwrapped once.
type body struct {
	lets     []string
	bindings map[string]string
	fallible bool
}

func (b *body) resolve(root protoreflect.MessageDescriptor, path string) (string, string, error) {
	segments := strings.Split(path, ".")
	hops, leaf := segments[:len(segments)-1], segments[len(segments)-1]

	message := root
	expr := "self"
	for depth, segment := range hops {
		field, err := lookup(message, segment)
		if err != nil {
			return "", "", err
		}
		if field.Kind() != protoreflect.MessageKind {
			return "", "", fmt.Errorf("`%s` is not a message field, so the path can't go through it", segment)
		}
		next := field.Message()
		if slices.Contains(wrappers, next.FullName()) {
			return "", "", fmt.Errorf("`%s` is a `%s`, which prost maps to an `Option` of a scalar, not a struct", segment, next.FullName())
		}
		prefix := strings.Join(segments[:depth+1], ".")
		if v, ok := b.bindings[prefix]; ok {
			expr = v
		} else {
			v := fmt.Sprintf("m%d", len(b.bindings))
			b.lets = append(b.lets, fmt.Sprintf("let %s = %s.%s.as_ref()?;", v, expr, fieldIdent(field)))
			b.bindings[prefix] = v
			b.fallible = true
			expr = v
		}
		message = next
	}

	field, err := lookup(message, leaf)
	if err != nil {
		return "", "", err
	}
	access := fmt.Sprintf("%s.%s", expr, fieldIdent(field))
	optional := field.HasPresence()

	var ty string
	byRef := false
	switch field.Kind() {
	case protoreflect.StringKind:
		ty, byRef = "&str", true
	case protoreflect.BytesKind:
		ty, byRef = "&[u8]", true
	case protoreflect.DoubleKind:
		ty = "f64"
	case protoreflect.FloatKind:
		ty = "f32"
	case protoreflect.Int32Kind, protoreflect.Sint32Kind, protoreflect.Sfixed32Kind:
		ty = "i32"
	case protoreflect.Int64Kind, protoreflect.Sint64Kind, protoreflect.Sfixed64Kind:
		ty = "i64"
	case protoreflect.Uint32Kind, protoreflect.Fixed32Kind:
		ty = "u32"
	case protoreflect.Uint64Kind, protoreflect.Fixed64Kind:
		ty = "u64"
	case protoreflect.BoolKind:
		ty = "bool"
	default:
		return "", "", fmt.Errorf("`%s` must be a scalar field (string, bytes, integer, bool or float)", leaf)
	}

	switch {
	case byRef && optional:
		expr = access + ".as_deref()?"
	case byRef:
		expr = "&*" + access
	case optional:
		expr = access + "?"
	default:
		expr = access
	}
	b.fallible = b.fallible || optional
	return ty, expr, nil
}

func lookup(message protoreflect.MessageDescriptor, name string) (protoreflect.FieldDescriptor, error) {
	field := message.Fields().ByName(protoreflect.Name(name))
	if field == nil {
		return nil, fmt.Errorf("`%s` has no field `%s`", message.FullName(), name)
	}
	if field.IsList() || field.IsMap() {
		return nil, fmt.Errorf("`%s` is repeated or a map; a pair side needs a single value", name)
	}
	if oneof := field.ContainingOneof(); oneof != nil && !oneof.IsSynthetic() {
		return nil, fmt.Errorf("`%s` is in a oneof, which prost models as an enum", name)
	}
	return field, nil
}

func rustPath(message protoreflect.MessageDescriptor) string {
	var segments []string
	for _, part := range strings.Split(string(message.ParentFile().Package()), ".") {
		segments = append(segments, sanitizeIdentifier(toSnakeCase(part)))
	}
	var parents []string
	for parent, ok := message.Parent().(protoreflect.MessageDescriptor); ok; parent, ok = parent.Parent().(protoreflect.MessageDescriptor) {
		parents = append(parents, sanitizeIdentifier(toSnakeCase(string(parent.Name()))))
	}
	slices.Reverse(parents)
	segments = append(segments, parents...)
	segments = append(segments, sanitizeIdentifier(toUpperCamelCase(string(message.Name()))))
	return "crate::" + strings.Join(segments, "::")
}

func fieldIdent(field protoreflect.FieldDescriptor) string {
	return sanitizeIdentifier(toSnakeCase(string(field.Name())))
}

// sanitizeIdentifier mirrors prost-build's ident::sanitize_identifier, so generated
// paths match prost's.
func sanitizeIdentifier(ident string) string {
	switch ident {
	case "as", "break", "const", "continue", "else", "enum", "false", "fn", "for", "if",
		"impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
		"return", "static", "struct", "trait", "true", "type", "unsafe", "use",
		"where", "while", "dyn", "abstract", "become", "box", "do", "final", "macro",
		"override", "priv", "typeof", "unsized", "virtual", "yield", "async",
		"await", "try", "gen":
		return "r#" + ident
	case "_", "super", "self", "Self", "extern", "crate":
		return ident + "_"
	}
	if ident != "" && ident[0] >= '0' && ident[0] <= '9' {
		return "_" + ident
	}
	return ident
}
