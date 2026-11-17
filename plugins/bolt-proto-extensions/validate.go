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
	fmt.Fprintf(&code, "\ncrate::ext::impl_validate! {\n    %s, %s, self;\n", rustPath(message), rustStr(string(message.FullName())))
	for _, r := range rules {
		field := r.field
		if oneof := field.ContainingOneof(); oneof != nil && !oneof.IsSynthetic() {
			return "", fmt.Errorf("field `%s` is in a oneof, which prost models as an enum, so it can't carry (bolt.options.v1.validate)", field.Name())
		}
		bind := "ref"
		if byValue(field) {
			bind = "val"
		}
		fmt.Fprintf(&code, "    %s %s %s {\n", fieldIdent(field), bind, rustStr(string(field.Name())))
		for _, expr := range r.exprs {
			expr = strings.TrimSpace(expr)
			if expr == "" {
				return "", fmt.Errorf("field `%s` has an empty (bolt.options.v1.validate) rule", field.Name())
			}
			if strings.ContainsRune(expr, '\r') {
				return "", fmt.Errorf("field `%s`: (bolt.options.v1.validate) rule contains a carriage return", field.Name())
			}
			fmt.Fprintf(&code, "        %s => %s,\n", rustStr(expr), expr)
		}
		code.WriteString("    }\n")
	}
	code.WriteString("}\n")
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
