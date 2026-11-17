package main

import (
	"fmt"
	"strings"
	"unicode/utf8"

	"google.golang.org/protobuf/reflect/protoreflect"
)

type fieldRules struct {
	field protoreflect.FieldDescriptor
	exprs []string
}

func validateRules(message protoreflect.MessageDescriptor, ext protoreflect.ExtensionDescriptor) ([]fieldRules, error) {
	var all []fieldRules
	fields := message.Fields()
	for i := range fields.Len() {
		field := fields.Get(i)
		payloads, err := extensionPayloads(field.Options(), ext)
		if err != nil {
			return nil, fmt.Errorf("field `%s`: %w", field.Name(), err)
		}
		if len(payloads) == 0 {
			continue
		}
		rules := fieldRules{field: field}
		for _, payload := range payloads {
			if !utf8.Valid(payload) {
				return nil, fmt.Errorf("field `%s`: (bolt.options.v1.validate) is not valid UTF-8", field.Name())
			}
			rules.exprs = append(rules.exprs, string(payload))
		}
		all = append(all, rules)
	}
	return all, nil
}

func renderValidate(message protoreflect.MessageDescriptor, rules []fieldRules) (string, error) {
	var code strings.Builder
	fmt.Fprintf(&code, "\nimpl %s {\n", rustPath(message))
	code.WriteString("    /// Checks the `(bolt.options.v1.validate)` rules on each field, in field order,\n")
	code.WriteString("    /// and returns the first that fails.\n")
	code.WriteString("    pub fn validate(&self) -> ::core::result::Result<(), ::prost::alloc::string::String> {\n")
	for _, r := range rules {
		field := r.field
		if oneof := field.ContainingOneof(); oneof != nil && !oneof.IsSynthetic() {
			return "", fmt.Errorf("field `%s` is in a oneof, which prost models as an enum, so it can't carry (bolt.options.v1.validate)", field.Name())
		}
		ident := fieldIdent(field)
		binding := "&self." + ident
		if byValue(field) {
			binding = "self." + ident
		}
		code.WriteString("        {\n")
		fmt.Fprintf(&code, "            let %s = %s;\n", ident, binding)
		for _, expr := range r.exprs {
			expr = strings.TrimSpace(expr)
			if expr == "" {
				return "", fmt.Errorf("field `%s` has an empty (bolt.options.v1.validate) rule", field.Name())
			}
			msg := fmt.Sprintf("%s.%s: failed `%s`; self = ", message.FullName(), field.Name(), expr)
			fmt.Fprintf(&code, "            if !(%s) {\n", expr)
			fmt.Fprintf(&code, "                return Err(::prost::alloc::format!(\"%s{:?}\", self));\n", rustFmtStr(msg))
			code.WriteString("            }\n")
		}
		code.WriteString("        }\n")
	}
	code.WriteString("        Ok(())\n    }\n}\n")
	return code.String(), nil
}

// byValue reports whether prost generates the field as a Copy scalar, so the rule
// can compare it directly (`n > 0`) instead of through a reference.
func byValue(field protoreflect.FieldDescriptor) bool {
	if field.IsList() || field.IsMap() || field.HasPresence() {
		return false
	}
	switch field.Kind() {
	case protoreflect.StringKind, protoreflect.BytesKind, protoreflect.MessageKind, protoreflect.GroupKind:
		return false
	}
	return true
}

// rustFmtStr escapes s for the body of a Rust `format!` string literal.
func rustFmtStr(s string) string {
	var b strings.Builder
	for _, r := range s {
		switch r {
		case '\\':
			b.WriteString(`\\`)
		case '"':
			b.WriteString(`\"`)
		case '\n':
			b.WriteString(`\n`)
		case '\r':
			b.WriteString(`\r`)
		case '\t':
			b.WriteString(`\t`)
		case '{':
			b.WriteString("{{")
		case '}':
			b.WriteString("}}")
		default:
			if r < 0x20 || r == 0x7f {
				fmt.Fprintf(&b, `\u{%x}`, r)
			} else {
				b.WriteRune(r)
			}
		}
	}
	return b.String()
}
