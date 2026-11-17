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

	baseSide, baseFallible, err := resolve(message, base)
	if err != nil {
		return "", fmt.Errorf("base path `%s`: %w", base, err)
	}
	quoteSide, quoteFallible, err := resolve(message, quote)
	if err != nil {
		return "", fmt.Errorf("quote path `%s`: %w", quote, err)
	}
	mode := "infallible"
	if baseFallible || quoteFallible {
		mode = "fallible"
	}

	var code strings.Builder
	fmt.Fprintf(&code, "\ncrate::ext::impl_pair! {\n    %s;\n", rustPath(message))
	fmt.Fprintf(&code, "    paths = \"%s\", \"%s\";\n", base, quote)
	fmt.Fprintf(&code, "    %s;\n", mode)
	fmt.Fprintf(&code, "    base: %s;\n", baseSide)
	fmt.Fprintf(&code, "    quote: %s;\n", quoteSide)
	code.WriteString("}\n")
	return code.String(), nil
}

func resolve(root protoreflect.MessageDescriptor, path string) (side string, fallible bool, err error) {
	segments := strings.Split(path, ".")
	hops, leaf := segments[:len(segments)-1], segments[len(segments)-1]

	message := root
	var idents []string
	for _, segment := range hops {
		field, err := lookup(message, segment)
		if err != nil {
			return "", false, err
		}
		if field.Kind() != protoreflect.MessageKind {
			return "", false, fmt.Errorf("`%s` is not a message field, so the path can't go through it", segment)
		}
		next := field.Message()
		if slices.Contains(wrappers, next.FullName()) {
			return "", false, fmt.Errorf("`%s` is a `%s`, which prost maps to an `Option` of a scalar, not a struct", segment, next.FullName())
		}
		idents = append(idents, fieldIdent(field))
		message = next
	}

	field, err := lookup(message, leaf)
	if err != nil {
		return "", false, err
	}
	switch field.Kind() {
	case protoreflect.MessageKind, protoreflect.GroupKind, protoreflect.EnumKind:
		return "", false, fmt.Errorf("`%s` must be a scalar field (string, bytes, integer, bool or float)", leaf)
	}
	presence := "req"
	if field.HasPresence() {
		presence = "opt"
	}
	side = fmt.Sprintf("[%s] %s: %s %s", strings.Join(idents, " "), fieldIdent(field), field.Kind(), presence)
	return side, len(hops) > 0 || field.HasPresence(), nil
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
