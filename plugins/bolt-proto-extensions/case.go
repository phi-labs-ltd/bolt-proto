package main

import (
	"strings"
	"unicode"
	"unicode/utf8"
)

type wordMode int

const (
	boundary wordMode = iota
	lowercase
	uppercase
)

// words splits a name the way heck 0.5 does, so toSnakeCase and toUpperCamelCase
// match the names prost generates.
func words(s string) []string {
	var out []string
	for _, word := range strings.FieldsFunc(s, func(r rune) bool {
		return !unicode.IsLetter(r) && !unicode.IsDigit(r)
	}) {
		runes := []rune(word)
		start := 0
		mode := boundary
		for i, c := range runes {
			if i+1 == len(runes) {
				out = append(out, string(runes[start:]))
				break
			}
			next := runes[i+1]
			nextMode := mode
			if unicode.IsLower(c) {
				nextMode = lowercase
			} else if unicode.IsUpper(c) {
				nextMode = uppercase
			}
			switch {
			case nextMode == lowercase && unicode.IsUpper(next):
				out = append(out, string(runes[start:i+1]))
				start, mode = i+1, boundary
			case mode == uppercase && unicode.IsUpper(c) && unicode.IsLower(next):
				out = append(out, string(runes[start:i]))
				start, mode = i, boundary
			default:
				mode = nextMode
			}
		}
	}
	return out
}

func toSnakeCase(s string) string {
	ws := words(s)
	for i, w := range ws {
		ws[i] = strings.ToLower(w)
	}
	return strings.Join(ws, "_")
}

func toUpperCamelCase(s string) string {
	var b strings.Builder
	for _, w := range words(s) {
		first, size := utf8.DecodeRuneInString(w)
		b.WriteRune(unicode.ToUpper(first))
		b.WriteString(strings.ToLower(w[size:]))
	}
	return b.String()
}
