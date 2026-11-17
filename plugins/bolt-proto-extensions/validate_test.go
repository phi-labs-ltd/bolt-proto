package main

import "testing"

func TestRustFmtStr(t *testing.T) {
	cases := map[string]string{
		"a.b: failed `!a.is_empty()`":   "a.b: failed `!a.is_empty()`",
		`x.iter().all(|v| { v != "" })`: `x.iter().all(|v| {{ v != \"\" }})`,
		"back\\slash\nnew\ttab\x01":     `back\\slash\nnew\ttab\u{1}`,
	}
	for in, want := range cases {
		if got := rustFmtStr(in); got != want {
			t.Errorf("rustFmtStr(%q) = %q, want %q", in, got, want)
		}
	}
}
